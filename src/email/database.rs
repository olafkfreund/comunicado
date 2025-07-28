use sqlx::{SqlitePool, Row, sqlite::SqlitePoolOptions};
use sqlx::migrate::MigrateDatabase;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use crate::imap::{ImapMessage, MessageFlag};
use crate::ui::EmailComposeData;
use thiserror::Error;

/// Database-related errors
#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Database connection error: {0}")]
    Connection(#[from] sqlx::Error),
    
    #[error("Migration error: {0}")]
    Migration(String),
    
    #[error("Query error: {0}")]
    Query(String),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("UUID error: {0}")]
    Uuid(#[from] uuid::Error),
    
    #[error("Date parsing error: {0}")]
    DateParse(#[from] chrono::ParseError),
}

pub type DatabaseResult<T> = Result<T, DatabaseError>;

/// Stored email message in the database
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StoredMessage {
    pub id: Uuid,
    pub account_id: String,
    pub folder_name: String,
    pub imap_uid: u32,
    pub message_id: Option<String>,
    pub thread_id: Option<String>,
    pub in_reply_to: Option<String>,
    pub references: Vec<String>,
    
    // Headers
    pub subject: String,
    pub from_addr: String,
    pub from_name: Option<String>,
    pub to_addrs: Vec<String>,
    pub cc_addrs: Vec<String>,
    pub bcc_addrs: Vec<String>,
    pub reply_to: Option<String>,
    pub date: DateTime<Utc>,
    
    // Content
    pub body_text: Option<String>,
    pub body_html: Option<String>,
    pub attachments: Vec<StoredAttachment>,
    
    // Metadata
    pub flags: Vec<String>,
    pub labels: Vec<String>,
    pub size: Option<u32>,
    pub priority: Option<String>,
    
    // Sync metadata
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_synced: DateTime<Utc>,
    pub sync_version: i64,
    pub is_draft: bool,
    pub is_deleted: bool,
}

/// Stored email attachment
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StoredAttachment {
    pub id: String,
    pub filename: String,
    pub content_type: String,
    pub size: u32,
    pub content_id: Option<String>,
    pub is_inline: bool,
    pub data: Option<Vec<u8>>, // Stored inline for small attachments
    pub file_path: Option<String>, // File path for large attachments
}

/// Stored folder in the database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredFolder {
    pub account_id: String,
    pub name: String,
    pub full_name: String,
    pub delimiter: Option<String>,
    pub attributes: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Folder synchronization state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderSyncState {
    pub account_id: String,
    pub folder_name: String,
    pub uid_validity: u32,
    pub uid_next: u32,
    pub highest_modseq: Option<u64>,
    pub last_sync: DateTime<Utc>,
    pub message_count: u32,
    pub unread_count: u32,
    pub sync_status: SyncStatus,
}

/// Synchronization status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncStatus {
    Idle,
    Syncing,
    Error(String),
    Complete,
}

/// Stored draft email in the database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredDraft {
    pub id: String,
    pub account_id: String,
    pub subject: String,
    pub to_addrs: Vec<String>,
    pub cc_addrs: Vec<String>,
    pub bcc_addrs: Vec<String>,
    pub reply_to: Option<String>,
    pub body_text: String,
    pub body_html: String,
    pub attachments: Vec<StoredAttachment>,
    pub in_reply_to: Option<String>,
    pub references: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub auto_saved: bool,
}

/// Email database manager
pub struct EmailDatabase {
    pub pool: SqlitePool,
    db_path: String,
}

impl EmailDatabase {
    /// Create a new email database
    pub async fn new(db_path: &str) -> DatabaseResult<Self> {
        // Create database if it doesn't exist
        if !sqlx::Sqlite::database_exists(db_path).await.unwrap_or(false) {
            sqlx::Sqlite::create_database(db_path).await
                .map_err(|e| DatabaseError::Migration(format!("Failed to create database: {}", e)))?;
        }
        
        // Create connection pool
        let pool = SqlitePoolOptions::new()
            .max_connections(20)
            .connect(db_path)
            .await
            .map_err(DatabaseError::Connection)?;
        
        let db = Self {
            pool,
            db_path: db_path.to_string(),
        };
        
        // Run migrations
        db.migrate().await?;
        
        Ok(db)
    }
    
    /// Create a new in-memory email database for testing
    pub async fn new_in_memory() -> DatabaseResult<Self> {
        // Create connection pool for in-memory database
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect(":memory:")
            .await
            .map_err(DatabaseError::Connection)?;
        
        let db = Self {
            pool,
            db_path: ":memory:".to_string(),
        };
        
        // Run migrations
        db.migrate().await?;
        
        Ok(db)
    }
    
    /// Run database migrations
    async fn migrate(&self) -> DatabaseResult<()> {
        // Enable foreign key constraints
        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&self.pool)
            .await?;
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS accounts (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                email TEXT NOT NULL,
                provider TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )
        "#).execute(&self.pool).await?;
        
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS folders (
                account_id TEXT NOT NULL,
                name TEXT NOT NULL,
                full_name TEXT NOT NULL,
                delimiter TEXT,
                attributes TEXT NOT NULL, -- JSON array
                uid_validity INTEGER,
                uid_next INTEGER,
                exists_count INTEGER,
                recent_count INTEGER,
                unseen_count INTEGER,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                PRIMARY KEY (account_id, name)
            )
        "#).execute(&self.pool).await?;
        
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS messages (
                id TEXT PRIMARY KEY,
                account_id TEXT NOT NULL,
                folder_name TEXT NOT NULL,
                imap_uid INTEGER NOT NULL,
                message_id TEXT,
                thread_id TEXT,
                in_reply_to TEXT,
                message_references TEXT NOT NULL, -- JSON array
                
                -- Headers
                subject TEXT NOT NULL,
                from_addr TEXT NOT NULL,
                from_name TEXT,
                to_addrs TEXT NOT NULL, -- JSON array
                cc_addrs TEXT NOT NULL, -- JSON array
                bcc_addrs TEXT NOT NULL, -- JSON array
                reply_to TEXT,
                date TEXT NOT NULL,
                
                -- Content
                body_text TEXT,
                body_html TEXT,
                attachments TEXT NOT NULL, -- JSON array
                
                -- Metadata
                flags TEXT NOT NULL, -- JSON array
                labels TEXT NOT NULL, -- JSON array
                size INTEGER,
                priority TEXT,
                
                -- Sync metadata
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                last_synced TEXT NOT NULL,
                sync_version INTEGER NOT NULL DEFAULT 1,
                is_draft BOOLEAN NOT NULL DEFAULT FALSE,
                is_deleted BOOLEAN NOT NULL DEFAULT FALSE
            )
        "#).execute(&self.pool).await?;
        
        // First, clean up any existing duplicates by keeping only the latest one
        // This needs to be done before adding the unique constraint
        let cleanup_result = sqlx::query(r#"
            DELETE FROM messages 
            WHERE rowid NOT IN (
                SELECT MIN(rowid) 
                FROM messages 
                GROUP BY account_id, folder_name, imap_uid
            )
        "#).execute(&self.pool).await;
        
        if let Ok(result) = cleanup_result {
            if result.rows_affected() > 0 {
                tracing::info!("Cleaned up {} duplicate messages", result.rows_affected());
            }
        }
        
        // Now add unique constraint to prevent future duplicates
        sqlx::query(r#"
            CREATE UNIQUE INDEX IF NOT EXISTS idx_messages_unique 
            ON messages (account_id, folder_name, imap_uid)
        "#).execute(&self.pool).await?;
        
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS folder_sync_state (
                account_id TEXT NOT NULL,
                folder_name TEXT NOT NULL,
                uid_validity INTEGER NOT NULL,
                uid_next INTEGER NOT NULL,
                highest_modseq INTEGER,
                last_sync TEXT NOT NULL,
                message_count INTEGER NOT NULL DEFAULT 0,
                unread_count INTEGER NOT NULL DEFAULT 0,
                sync_status TEXT NOT NULL,
                PRIMARY KEY (account_id, folder_name)
            )
        "#).execute(&self.pool).await?;
        
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS email_filters (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT NOT NULL,
                enabled BOOLEAN NOT NULL DEFAULT TRUE,
                priority INTEGER NOT NULL DEFAULT 100,
                conditions TEXT NOT NULL, -- JSON array of FilterCondition
                actions TEXT NOT NULL, -- JSON array of FilterAction
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )
        "#).execute(&self.pool).await?;
        
        // Create drafts table
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS drafts (
                id TEXT PRIMARY KEY,
                account_id TEXT NOT NULL,
                subject TEXT NOT NULL DEFAULT '',
                to_addrs TEXT NOT NULL DEFAULT '', -- JSON array
                cc_addrs TEXT NOT NULL DEFAULT '', -- JSON array  
                bcc_addrs TEXT NOT NULL DEFAULT '', -- JSON array
                reply_to TEXT,
                body_text TEXT NOT NULL DEFAULT '',
                body_html TEXT NOT NULL DEFAULT '',
                attachments TEXT NOT NULL DEFAULT '', -- JSON array of attachment info
                in_reply_to TEXT, -- Message ID if this is a reply
                draft_references TEXT NOT NULL DEFAULT '', -- JSON array of Message IDs
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                auto_saved BOOLEAN NOT NULL DEFAULT FALSE
            )
        "#).execute(&self.pool).await?;
        
        // Create indexes for performance
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_messages_account_folder ON messages(account_id, folder_name)").execute(&self.pool).await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_messages_uid ON messages(account_id, folder_name, imap_uid)").execute(&self.pool).await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_messages_message_id ON messages(message_id)").execute(&self.pool).await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_messages_thread_id ON messages(thread_id)").execute(&self.pool).await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_messages_date ON messages(date DESC)").execute(&self.pool).await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_messages_subject ON messages(subject)").execute(&self.pool).await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_messages_from ON messages(from_addr)").execute(&self.pool).await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_messages_sync ON messages(last_synced)").execute(&self.pool).await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_email_filters_priority ON email_filters(priority, enabled)").execute(&self.pool).await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_drafts_account ON drafts(account_id)").execute(&self.pool).await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_drafts_updated ON drafts(updated_at DESC)").execute(&self.pool).await?;
        
        // Full-text search virtual table
        sqlx::query(r#"
            CREATE VIRTUAL TABLE IF NOT EXISTS messages_fts USING fts5(
                message_id UNINDEXED,
                subject,
                from_addr,
                from_name,
                body_text,
                content='messages',
                content_rowid='rowid'
            )
        "#).execute(&self.pool).await?;
        
        // Triggers to keep FTS table in sync
        sqlx::query(r#"
            CREATE TRIGGER IF NOT EXISTS messages_fts_insert AFTER INSERT ON messages BEGIN
                INSERT INTO messages_fts(rowid, message_id, subject, from_addr, from_name, body_text)
                VALUES (new.rowid, new.message_id, new.subject, new.from_addr, new.from_name, new.body_text);
            END
        "#).execute(&self.pool).await?;
        
        sqlx::query(r#"
            CREATE TRIGGER IF NOT EXISTS messages_fts_delete AFTER DELETE ON messages BEGIN
                INSERT INTO messages_fts(messages_fts, rowid, message_id, subject, from_addr, from_name, body_text)
                VALUES ('delete', old.rowid, old.message_id, old.subject, old.from_addr, old.from_name, old.body_text);
            END
        "#).execute(&self.pool).await?;
        
        sqlx::query(r#"
            CREATE TRIGGER IF NOT EXISTS messages_fts_update AFTER UPDATE ON messages BEGIN
                INSERT INTO messages_fts(messages_fts, rowid, message_id, subject, from_addr, from_name, body_text)
                VALUES ('delete', old.rowid, old.message_id, old.subject, old.from_addr, old.from_name, old.body_text);
                INSERT INTO messages_fts(rowid, message_id, subject, from_addr, from_name, body_text)
                VALUES (new.rowid, new.message_id, new.subject, new.from_addr, new.from_name, new.body_text);
            END
        "#).execute(&self.pool).await?;
        
        Ok(())
    }
    
    /// Store a message in the database
    pub async fn store_message(&self, message: &StoredMessage) -> DatabaseResult<()> {
        let now = Utc::now().to_rfc3339();
        
        // First try to insert, if it fails due to unique constraint, update instead
        let insert_result = sqlx::query(r#"
            INSERT INTO messages (
                id, account_id, folder_name, imap_uid, message_id, thread_id, in_reply_to, message_references,
                subject, from_addr, from_name, to_addrs, cc_addrs, bcc_addrs, reply_to, date,
                body_text, body_html, attachments,
                flags, labels, size, priority,
                created_at, updated_at, last_synced, sync_version, is_draft, is_deleted
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8,
                ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16,
                ?17, ?18, ?19,
                ?20, ?21, ?22, ?23,
                ?24, ?25, ?26, ?27, ?28, ?29
            )
        "#)
        .bind(message.id.to_string())
        .bind(&message.account_id)
        .bind(&message.folder_name)
        .bind(message.imap_uid as i64)
        .bind(&message.message_id)
        .bind(&message.thread_id)
        .bind(&message.in_reply_to)
        .bind(serde_json::to_string(&message.references)?)
        .bind(&message.subject)
        .bind(&message.from_addr)
        .bind(&message.from_name)
        .bind(serde_json::to_string(&message.to_addrs)?)
        .bind(serde_json::to_string(&message.cc_addrs)?)
        .bind(serde_json::to_string(&message.bcc_addrs)?)
        .bind(&message.reply_to)
        .bind(message.date.to_rfc3339())
        .bind(&message.body_text)
        .bind(&message.body_html)
        .bind(serde_json::to_string(&message.attachments)?)
        .bind(serde_json::to_string(&message.flags)?)
        .bind(serde_json::to_string(&message.labels)?)
        .bind(message.size.map(|s| s as i64))
        .bind(&message.priority)
        .bind(message.created_at.to_rfc3339())
        .bind(now.clone())
        .bind(message.last_synced.to_rfc3339())
        .bind(message.sync_version)
        .bind(message.is_draft)
        .bind(message.is_deleted)
        .execute(&self.pool)
        .await;
        
        match insert_result {
            Ok(_) => {
                tracing::debug!("Successfully inserted message UID {} for account {} in folder {}", 
                    message.imap_uid, message.account_id, message.folder_name);
                Ok(())
            }
            Err(sqlx::Error::Database(db_err)) if db_err.message().contains("UNIQUE constraint failed") => {
                tracing::debug!("Message UID {} already exists, updating instead", message.imap_uid);
                // Update the existing message
                sqlx::query(r#"
                    UPDATE messages SET
                        message_id = ?1, thread_id = ?2, in_reply_to = ?3, message_references = ?4,
                        subject = ?5, from_addr = ?6, from_name = ?7, to_addrs = ?8, cc_addrs = ?9, 
                        bcc_addrs = ?10, reply_to = ?11, date = ?12, body_text = ?13, body_html = ?14,
                        attachments = ?15, flags = ?16, labels = ?17, size = ?18, priority = ?19,
                        updated_at = ?20, last_synced = ?21, sync_version = ?22, is_draft = ?23, is_deleted = ?24
                    WHERE account_id = ?25 AND folder_name = ?26 AND imap_uid = ?27
                "#)
                .bind(&message.message_id)
                .bind(&message.thread_id)
                .bind(&message.in_reply_to)
                .bind(serde_json::to_string(&message.references)?)
                .bind(&message.subject)
                .bind(&message.from_addr)
                .bind(&message.from_name)
                .bind(serde_json::to_string(&message.to_addrs)?)
                .bind(serde_json::to_string(&message.cc_addrs)?)
                .bind(serde_json::to_string(&message.bcc_addrs)?)
                .bind(&message.reply_to)
                .bind(message.date.to_rfc3339())
                .bind(&message.body_text)
                .bind(&message.body_html)
                .bind(serde_json::to_string(&message.attachments)?)
                .bind(serde_json::to_string(&message.flags)?)
                .bind(serde_json::to_string(&message.labels)?)
                .bind(message.size.map(|s| s as i64))
                .bind(&message.priority)
                .bind(now)
                .bind(message.last_synced.to_rfc3339())
                .bind(message.sync_version)
                .bind(message.is_draft)
                .bind(message.is_deleted)
                .bind(&message.account_id)
                .bind(&message.folder_name)
                .bind(message.imap_uid as i64)
                .execute(&self.pool)
                .await?;
                
                Ok(())
            }
            Err(e) => Err(DatabaseError::Connection(e))
        }
    }
    
    /// Get messages from a folder
    pub async fn get_messages(&self, account_id: &str, folder_name: &str, limit: Option<u32>, offset: Option<u32>) -> DatabaseResult<Vec<StoredMessage>> {
        let limit = limit.unwrap_or(100) as i64;
        let offset = offset.unwrap_or(0) as i64;
        
        let rows = sqlx::query(r#"
            SELECT id, account_id, folder_name, imap_uid, message_id, thread_id, in_reply_to, message_references,
                   subject, from_addr, from_name, to_addrs, cc_addrs, bcc_addrs, reply_to, date,
                   body_text, body_html, attachments,
                   flags, labels, size, priority,
                   created_at, updated_at, last_synced, sync_version, is_draft, is_deleted
            FROM messages
            WHERE account_id = ?1 AND folder_name = ?2 AND is_deleted = FALSE
            ORDER BY date DESC
            LIMIT ?3 OFFSET ?4
        "#)
        .bind(account_id)
        .bind(folder_name)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        
        let mut messages = Vec::new();
        for row in rows {
            messages.push(self.row_to_stored_message(row)?);
        }
        
        Ok(messages)
    }
    
    /// Get a message by UID
    pub async fn get_message_by_uid(&self, account_id: &str, folder_name: &str, uid: u32) -> DatabaseResult<Option<StoredMessage>> {
        let row = sqlx::query(r#"
            SELECT id, account_id, folder_name, imap_uid, message_id, thread_id, in_reply_to, message_references,
                   subject, from_addr, from_name, to_addrs, cc_addrs, bcc_addrs, reply_to, date,
                   body_text, body_html, attachments,
                   flags, labels, size, priority,
                   created_at, updated_at, last_synced, sync_version, is_draft, is_deleted
            FROM messages
            WHERE account_id = ?1 AND folder_name = ?2 AND imap_uid = ?3
        "#)
        .bind(account_id)
        .bind(folder_name)
        .bind(uid as i64)
        .fetch_optional(&self.pool)
        .await?;
        
        match row {
            Some(row) => Ok(Some(self.row_to_stored_message(row)?)),
            None => Ok(None),
        }
    }
    
    /// Search messages with full-text search
    pub async fn search_messages(&self, account_id: &str, query: &str, limit: Option<u32>) -> DatabaseResult<Vec<StoredMessage>> {
        let limit = limit.unwrap_or(100) as i64;
        
        let rows = sqlx::query(r#"
            SELECT m.id, m.account_id, m.folder_name, m.imap_uid, m.message_id, m.thread_id, m.in_reply_to, m.message_references,
                   m.subject, m.from_addr, m.from_name, m.to_addrs, m.cc_addrs, m.bcc_addrs, m.reply_to, m.date,
                   m.body_text, m.body_html, m.attachments,
                   m.flags, m.labels, m.size, m.priority,
                   m.created_at, m.updated_at, m.last_synced, m.sync_version, m.is_draft, m.is_deleted
            FROM messages m
            JOIN messages_fts fts ON m.rowid = fts.rowid
            WHERE m.account_id = ?1 AND m.is_deleted = FALSE AND messages_fts MATCH ?2
            ORDER BY rank
            LIMIT ?3
        "#)
        .bind(account_id)
        .bind(query)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        
        let mut messages = Vec::new();
        for row in rows {
            messages.push(self.row_to_stored_message(row)?);
        }
        
        Ok(messages)
    }
    
    /// Update folder sync state
    pub async fn update_folder_sync_state(&self, state: &FolderSyncState) -> DatabaseResult<()> {
        sqlx::query(r#"
            INSERT OR REPLACE INTO folder_sync_state (
                account_id, folder_name, uid_validity, uid_next, highest_modseq,
                last_sync, message_count, unread_count, sync_status
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
        "#)
        .bind(&state.account_id)
        .bind(&state.folder_name)
        .bind(state.uid_validity as i64)
        .bind(state.uid_next as i64)
        .bind(state.highest_modseq.map(|m| m as i64))
        .bind(state.last_sync.to_rfc3339())
        .bind(state.message_count as i64)
        .bind(state.unread_count as i64)
        .bind(serde_json::to_string(&state.sync_status)?)
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }
    
    /// Get all folders for an account
    pub async fn get_folders(&self, account_id: &str) -> DatabaseResult<Vec<StoredFolder>> {
        let rows = sqlx::query(r#"
            SELECT account_id, name, full_name, delimiter, attributes, created_at, updated_at
            FROM folders
            WHERE account_id = ?1
            ORDER BY name
        "#)
        .bind(account_id)
        .fetch_all(&self.pool)
        .await?;
        
        let mut folders = Vec::new();
        for row in rows {
            let attributes_json: String = row.get("attributes");
            let attributes: Vec<String> = serde_json::from_str(&attributes_json)
                .unwrap_or_else(|_| Vec::new());
            let created_at: DateTime<Utc> = DateTime::parse_from_rfc3339(row.get("created_at"))?.into();
            let updated_at: DateTime<Utc> = DateTime::parse_from_rfc3339(row.get("updated_at"))?.into();
            
            folders.push(StoredFolder {
                account_id: row.get("account_id"),
                name: row.get("name"),
                full_name: row.get("full_name"),
                delimiter: Some(row.get("delimiter")),
                attributes,
                created_at,
                updated_at,
            });
        }
        
        Ok(folders)
    }

    /// Get folder sync state
    pub async fn get_folder_sync_state(&self, account_id: &str, folder_name: &str) -> DatabaseResult<Option<FolderSyncState>> {
        let row = sqlx::query(r#"
            SELECT account_id, folder_name, uid_validity, uid_next, highest_modseq,
                   last_sync, message_count, unread_count, sync_status
            FROM folder_sync_state
            WHERE account_id = ?1 AND folder_name = ?2
        "#)
        .bind(account_id)
        .bind(folder_name)
        .fetch_optional(&self.pool)
        .await?;
        
        match row {
            Some(row) => {
                let sync_status: SyncStatus = serde_json::from_str(row.get("sync_status"))?;
                let last_sync: DateTime<Utc> = DateTime::parse_from_rfc3339(row.get("last_sync"))?.into();
                
                Ok(Some(FolderSyncState {
                    account_id: row.get("account_id"),
                    folder_name: row.get("folder_name"),
                    uid_validity: row.get::<i64, _>("uid_validity") as u32,
                    uid_next: row.get::<i64, _>("uid_next") as u32,
                    highest_modseq: row.get::<Option<i64>, _>("highest_modseq").map(|m| m as u64),
                    last_sync,
                    message_count: row.get::<i64, _>("message_count") as u32,
                    unread_count: row.get::<i64, _>("unread_count") as u32,
                    sync_status,
                }))
            }
            None => Ok(None),
        }
    }
    
    /// Delete messages by UIDs
    pub async fn delete_messages_by_uids(&self, account_id: &str, folder_name: &str, uids: &[u32]) -> DatabaseResult<()> {
        for uid in uids {
            sqlx::query("UPDATE messages SET is_deleted = TRUE, updated_at = ?1 WHERE account_id = ?2 AND folder_name = ?3 AND imap_uid = ?4")
                .bind(Utc::now().to_rfc3339())
                .bind(account_id)
                .bind(folder_name)
                .bind(*uid as i64)
                .execute(&self.pool)
                .await?;
        }
        
        Ok(())
    }
    
    /// Get database statistics
    pub async fn get_stats(&self) -> DatabaseResult<DatabaseStats> {
        let message_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM messages WHERE is_deleted = FALSE")
            .fetch_one(&self.pool).await?;
            
        let unread_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM messages WHERE is_deleted = FALSE AND flags NOT LIKE '%\"\\\\Seen\"%'")
            .fetch_one(&self.pool).await?;
            
        let account_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM accounts")
            .fetch_one(&self.pool).await?;
            
        let folder_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM folders")
            .fetch_one(&self.pool).await?;
        
        let db_size = std::fs::metadata(&self.db_path)
            .map(|metadata| metadata.len())
            .unwrap_or(0);
        
        Ok(DatabaseStats {
            message_count: message_count as u32,
            unread_count: unread_count as u32,
            account_count: account_count as u32,
            folder_count: folder_count as u32,
            db_size_bytes: db_size,
        })
    }
    
    /// Helper to convert database row to StoredMessage
    pub fn row_to_stored_message(&self, row: sqlx::sqlite::SqliteRow) -> DatabaseResult<StoredMessage> {
        let id = Uuid::parse_str(row.get("id"))?;
        let references: Vec<String> = serde_json::from_str(row.get("message_references"))?;
        let to_addrs: Vec<String> = serde_json::from_str(row.get("to_addrs"))?;
        let cc_addrs: Vec<String> = serde_json::from_str(row.get("cc_addrs"))?;
        let bcc_addrs: Vec<String> = serde_json::from_str(row.get("bcc_addrs"))?;
        let attachments: Vec<StoredAttachment> = serde_json::from_str(row.get("attachments"))?;
        let flags: Vec<String> = serde_json::from_str(row.get("flags"))?;
        let labels: Vec<String> = serde_json::from_str(row.get("labels"))?;
        
        let date: DateTime<Utc> = DateTime::parse_from_rfc3339(row.get("date"))?.into();
        let created_at: DateTime<Utc> = DateTime::parse_from_rfc3339(row.get("created_at"))?.into();
        let updated_at: DateTime<Utc> = DateTime::parse_from_rfc3339(row.get("updated_at"))?.into();
        let last_synced: DateTime<Utc> = DateTime::parse_from_rfc3339(row.get("last_synced"))?.into();
        
        Ok(StoredMessage {
            id,
            account_id: row.get("account_id"),
            folder_name: row.get("folder_name"),
            imap_uid: row.get::<i64, _>("imap_uid") as u32,
            message_id: row.get("message_id"),
            thread_id: row.get("thread_id"),
            in_reply_to: row.get("in_reply_to"),
            references,
            subject: row.get("subject"),
            from_addr: row.get("from_addr"),
            from_name: row.get("from_name"),
            to_addrs,
            cc_addrs,
            bcc_addrs,
            reply_to: row.get("reply_to"),
            date,
            body_text: row.get("body_text"),
            body_html: row.get("body_html"),
            attachments,
            flags,
            labels,
            size: row.get::<Option<i64>, _>("size").map(|s| s as u32),
            priority: row.get("priority"),
            created_at,
            updated_at,
            last_synced,
            sync_version: row.get("sync_version"),
            is_draft: row.get("is_draft"),
            is_deleted: row.get("is_deleted"),
        })
    }
    
    /// Store an email filter
    pub async fn store_filter(&self, filter: &crate::email::EmailFilter) -> DatabaseResult<()> {
        let conditions_json = serde_json::to_string(&filter.conditions)?;
        let actions_json = serde_json::to_string(&filter.actions)?;
        
        sqlx::query(r#"
            INSERT OR REPLACE INTO email_filters (
                id, name, description, enabled, priority,
                conditions, actions, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
        "#)
        .bind(filter.id.to_string())
        .bind(&filter.name)
        .bind(&filter.description)
        .bind(filter.enabled)
        .bind(filter.priority)
        .bind(conditions_json)
        .bind(actions_json)
        .bind(filter.created_at.to_rfc3339())
        .bind(filter.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }
    
    /// Get all email filters
    pub async fn get_filters(&self) -> DatabaseResult<Vec<crate::email::EmailFilter>> {
        let rows = sqlx::query(r#"
            SELECT id, name, description, enabled, priority,
                   conditions, actions, created_at, updated_at
            FROM email_filters
            ORDER BY priority ASC, created_at ASC
        "#)
        .fetch_all(&self.pool)
        .await?;
        
        let mut filters = Vec::new();
        for row in rows {
            let id = uuid::Uuid::parse_str(row.get("id"))?;
            let conditions_json: String = row.get("conditions");
            let actions_json: String = row.get("actions");
            let conditions = serde_json::from_str(&conditions_json)?;
            let actions = serde_json::from_str(&actions_json)?;
            
            let created_at: DateTime<Utc> = DateTime::parse_from_rfc3339(row.get("created_at"))?.into();
            let updated_at: DateTime<Utc> = DateTime::parse_from_rfc3339(row.get("updated_at"))?.into();
            
            filters.push(crate::email::EmailFilter {
                id,
                name: row.get("name"),
                description: row.get("description"),
                enabled: row.get("enabled"),
                priority: row.get("priority"),
                conditions,
                actions,
                created_at,
                updated_at,
            });
        }
        
        Ok(filters)
    }
    
    /// Get a specific filter by ID
    pub async fn get_filter(&self, filter_id: uuid::Uuid) -> DatabaseResult<Option<crate::email::EmailFilter>> {
        let row = sqlx::query(r#"
            SELECT id, name, description, enabled, priority,
                   conditions, actions, created_at, updated_at
            FROM email_filters
            WHERE id = ?1
        "#)
        .bind(filter_id.to_string())
        .fetch_optional(&self.pool)
        .await?;
        
        if let Some(row) = row {
            let id = uuid::Uuid::parse_str(row.get("id"))?;
            let conditions_json: String = row.get("conditions");
            let actions_json: String = row.get("actions");
            let conditions = serde_json::from_str(&conditions_json)?;
            let actions = serde_json::from_str(&actions_json)?;
            
            let created_at: DateTime<Utc> = DateTime::parse_from_rfc3339(row.get("created_at"))?.into();
            let updated_at: DateTime<Utc> = DateTime::parse_from_rfc3339(row.get("updated_at"))?.into();
            
            Ok(Some(crate::email::EmailFilter {
                id,
                name: row.get("name"),
                description: row.get("description"),
                enabled: row.get("enabled"),
                priority: row.get("priority"),
                conditions,
                actions,
                created_at,
                updated_at,
            }))
        } else {
            Ok(None)
        }
    }
    
    /// Delete a filter by ID
    pub async fn delete_filter(&self, filter_id: uuid::Uuid) -> DatabaseResult<()> {
        sqlx::query("DELETE FROM email_filters WHERE id = ?1")
            .bind(filter_id.to_string())
            .execute(&self.pool)
            .await?;
        
        Ok(())
    }
    
    /// Update filter enabled status
    pub async fn set_filter_enabled(&self, filter_id: uuid::Uuid, enabled: bool) -> DatabaseResult<()> {
        sqlx::query(r#"
            UPDATE email_filters 
            SET enabled = ?1, updated_at = ?2
            WHERE id = ?3
        "#)
        .bind(enabled)
        .bind(chrono::Utc::now().to_rfc3339())
        .bind(filter_id.to_string())
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }
    
    /// Save a draft to the database
    pub async fn save_draft(&self, draft: &StoredDraft) -> DatabaseResult<()> {
        let to_addrs_json = serde_json::to_string(&draft.to_addrs)?;
        let cc_addrs_json = serde_json::to_string(&draft.cc_addrs)?;
        let bcc_addrs_json = serde_json::to_string(&draft.bcc_addrs)?;
        let attachments_json = serde_json::to_string(&draft.attachments)?;
        let references_json = serde_json::to_string(&draft.references)?;
        
        sqlx::query(r#"
            INSERT OR REPLACE INTO drafts (
                id, account_id, subject, to_addrs, cc_addrs, bcc_addrs, reply_to,
                body_text, body_html, attachments, in_reply_to, draft_references,
                created_at, updated_at, auto_saved
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#)
        .bind(&draft.id)
        .bind(&draft.account_id)
        .bind(&draft.subject)
        .bind(&to_addrs_json)
        .bind(&cc_addrs_json)
        .bind(&bcc_addrs_json)
        .bind(&draft.reply_to)
        .bind(&draft.body_text)
        .bind(&draft.body_html)
        .bind(&attachments_json)
        .bind(&draft.in_reply_to)
        .bind(&references_json)
        .bind(draft.created_at.to_rfc3339())
        .bind(draft.updated_at.to_rfc3339())
        .bind(draft.auto_saved)
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }
    
    /// Load a draft by ID
    pub async fn load_draft(&self, draft_id: &str) -> DatabaseResult<Option<StoredDraft>> {
        let row = sqlx::query(r#"
            SELECT id, account_id, subject, to_addrs, cc_addrs, bcc_addrs, reply_to,
                   body_text, body_html, attachments, in_reply_to, draft_references,
                   created_at, updated_at, auto_saved
            FROM drafts WHERE id = ?
        "#)
        .bind(draft_id)
        .fetch_optional(&self.pool)
        .await?;
        
        if let Some(row) = row {
            let to_addrs: Vec<String> = serde_json::from_str(&row.get::<String, _>("to_addrs"))?;
            let cc_addrs: Vec<String> = serde_json::from_str(&row.get::<String, _>("cc_addrs"))?;
            let bcc_addrs: Vec<String> = serde_json::from_str(&row.get::<String, _>("bcc_addrs"))?;
            let attachments: Vec<StoredAttachment> = serde_json::from_str(&row.get::<String, _>("attachments"))?;
            let references: Vec<String> = serde_json::from_str(&row.get::<String, _>("draft_references"))?;
            
            let created_at_str: String = row.get("created_at");
            let updated_at_str: String = row.get("updated_at");
            
            Ok(Some(StoredDraft {
                id: row.get("id"),
                account_id: row.get("account_id"),
                subject: row.get("subject"),
                to_addrs,
                cc_addrs,
                bcc_addrs,
                reply_to: row.get("reply_to"),
                body_text: row.get("body_text"),
                body_html: row.get("body_html"),
                attachments,
                in_reply_to: row.get("in_reply_to"),
                references,
                created_at: DateTime::parse_from_rfc3339(&created_at_str)?.with_timezone(&Utc),
                updated_at: DateTime::parse_from_rfc3339(&updated_at_str)?.with_timezone(&Utc),
                auto_saved: row.get("auto_saved"),
            }))
        } else {
            Ok(None)
        }
    }
    
    /// Load all drafts for an account
    pub async fn load_drafts_for_account(&self, account_id: &str) -> DatabaseResult<Vec<StoredDraft>> {
        let rows = sqlx::query(r#"
            SELECT id, account_id, subject, to_addrs, cc_addrs, bcc_addrs, reply_to,
                   body_text, body_html, attachments, in_reply_to, draft_references,
                   created_at, updated_at, auto_saved
            FROM drafts WHERE account_id = ?
            ORDER BY updated_at DESC
        "#)
        .bind(account_id)
        .fetch_all(&self.pool)
        .await?;
        
        let mut drafts = Vec::new();
        for row in rows {
            let to_addrs: Vec<String> = serde_json::from_str(&row.get::<String, _>("to_addrs"))?;
            let cc_addrs: Vec<String> = serde_json::from_str(&row.get::<String, _>("cc_addrs"))?;
            let bcc_addrs: Vec<String> = serde_json::from_str(&row.get::<String, _>("bcc_addrs"))?;
            let attachments: Vec<StoredAttachment> = serde_json::from_str(&row.get::<String, _>("attachments"))?;
            let references: Vec<String> = serde_json::from_str(&row.get::<String, _>("draft_references"))?;
            
            let created_at_str: String = row.get("created_at");
            let updated_at_str: String = row.get("updated_at");
            
            drafts.push(StoredDraft {
                id: row.get("id"),
                account_id: row.get("account_id"),
                subject: row.get("subject"),
                to_addrs,
                cc_addrs,
                bcc_addrs,
                reply_to: row.get("reply_to"),
                body_text: row.get("body_text"),
                body_html: row.get("body_html"),
                attachments,
                in_reply_to: row.get("in_reply_to"),
                references,
                created_at: DateTime::parse_from_rfc3339(&created_at_str)?.with_timezone(&Utc),
                updated_at: DateTime::parse_from_rfc3339(&updated_at_str)?.with_timezone(&Utc),
                auto_saved: row.get("auto_saved"),
            });
        }
        
        Ok(drafts)
    }
    
    /// Delete a draft by ID
    pub async fn delete_draft(&self, draft_id: &str) -> DatabaseResult<bool> {
        let result = sqlx::query("DELETE FROM drafts WHERE id = ?")
            .bind(draft_id)
            .execute(&self.pool)
            .await?;
        
        Ok(result.rows_affected() > 0)
    }
    
    /// Delete all auto-saved drafts older than the specified duration
    pub async fn cleanup_old_auto_saved_drafts(&self, older_than_hours: i64) -> DatabaseResult<u64> {
        let cutoff_time = Utc::now() - chrono::Duration::hours(older_than_hours);
        
        let result = sqlx::query(r#"
            DELETE FROM drafts 
            WHERE auto_saved = TRUE AND updated_at < ?
        "#)
        .bind(cutoff_time.to_rfc3339())
        .execute(&self.pool)
        .await?;
        
        Ok(result.rows_affected())
    }
    
    /// Get draft statistics for an account
    pub async fn get_draft_stats(&self, account_id: &str) -> DatabaseResult<(u32, u32)> {
        let row = sqlx::query(r#"
            SELECT 
                COUNT(*) as total_count,
                SUM(CASE WHEN auto_saved = FALSE THEN 1 ELSE 0 END) as manual_count
            FROM drafts WHERE account_id = ?
        "#)
        .bind(account_id)
        .fetch_one(&self.pool)
        .await?;
        
        let total_count: i64 = row.get("total_count");
        let manual_count: i64 = row.get("manual_count");
        
        Ok((total_count as u32, manual_count as u32))
    }
    
    /// Clean and reprocess existing message content in the database
    /// This method fixes HTML/CSS and email headers that may still be present
    /// in previously stored message content
    pub async fn reprocess_message_content(&self) -> DatabaseResult<u32> {
        tracing::info!("Starting message content reprocessing to clean HTML and headers");
        
        // Get all messages that might need content cleaning
        let rows = sqlx::query(r#"
            SELECT id, body_text, body_html
            FROM messages 
            WHERE is_deleted = FALSE 
            AND (
                body_text LIKE '%<html%' OR 
                body_text LIKE '%<div%' OR 
                body_text LIKE '%content-type:%' OR
                body_text LIKE '%delivered-to:%' OR
                body_text LIKE '%authentication-results:%' OR
                body_text LIKE '%received:%' OR
                body_html LIKE '%<html%' OR 
                body_html LIKE '%<div%' OR 
                body_html LIKE '%content-type:%' OR
                body_html LIKE '%delivered-to:%' OR
                body_html LIKE '%authentication-results:%'
            )
            ORDER BY id
        "#)
        .fetch_all(&self.pool)
        .await?;
        
        let mut processed_count = 0;
        
        for row in rows {
            let message_id: String = row.get("id");
            let current_body_text: Option<String> = row.get("body_text");
            let current_body_html: Option<String> = row.get("body_html");
            
            let mut needs_update = false;
            let mut new_body_text = current_body_text.clone();
            let mut new_body_html = current_body_html.clone();
            
            // Check if body_text needs reprocessing (contains HTML tags or headers)  
            if let Some(body_text) = &current_body_text {
                if Self::content_needs_reprocessing(body_text) {
                    tracing::info!("Reprocessing body_text for message {}", message_id);
                    new_body_text = StoredMessage::parse_and_clean_body_text(&Some(body_text.clone()));
                    needs_update = true;
                }
            }
            
            // Check if body_html needs reprocessing 
            if let Some(body_html) = &current_body_html {
                if Self::content_needs_reprocessing(body_html) {
                    tracing::info!("Reprocessing body_html for message {}", message_id);
                    new_body_html = StoredMessage::parse_and_clean_body_html(&Some(body_html.clone()));
                    needs_update = true;
                }
            }
            
            // Update the message if content was reprocessed
            if needs_update {
                sqlx::query("UPDATE messages SET body_text = ?, body_html = ?, updated_at = datetime('now') WHERE id = ?")
                    .bind(&new_body_text)
                    .bind(&new_body_html) 
                    .bind(message_id)
                    .execute(&self.pool)
                    .await?;
                    
                processed_count += 1;
            }
        }
        
        tracing::info!("Completed message content reprocessing. {} messages updated", processed_count);
        Ok(processed_count)
    }
    
    /// Check if content needs reprocessing (contains HTML tags or email headers)
    fn content_needs_reprocessing(content: &str) -> bool {
        // Check for HTML tags
        let has_html = content.contains("<html") || 
                      content.contains("<div") ||
                      content.contains("<span") ||
                      content.contains("<p>") ||
                      content.contains("<br") ||
                      content.contains("<table") ||
                      content.contains("<td") ||
                      content.contains("<tr");
        
        // Check for email headers that shouldn't be in displayed content
        let has_headers = content.contains("content-type:") ||
                         content.contains("delivered-to:") ||
                         content.contains("authentication-results:") ||
                         content.contains("received:") ||
                         content.contains("dkim-signature:") ||
                         content.contains("mime-version:") ||
                         content.contains("content-transfer-encoding:");
        
        has_html || has_headers
    }
}

/// Database statistics
#[derive(Debug, Clone)]
pub struct DatabaseStats {
    pub message_count: u32,
    pub unread_count: u32,
    pub account_count: u32,
    pub folder_count: u32,
    pub db_size_bytes: u64,
}

/// Convert between EmailComposeData and StoredDraft
impl StoredDraft {
    /// Create a new draft from compose data
    pub fn from_compose_data(
        account_id: String,
        compose_data: &EmailComposeData,
        auto_saved: bool,
    ) -> Self {
        let now = Utc::now();
        
        Self {
            id: Uuid::new_v4().to_string(),
            account_id,
            subject: compose_data.subject.clone(),
            to_addrs: EmailComposeData::parse_addresses(&compose_data.to),
            cc_addrs: EmailComposeData::parse_addresses(&compose_data.cc),
            bcc_addrs: EmailComposeData::parse_addresses(&compose_data.bcc),
            reply_to: None, // TODO: Extract from compose data if available
            body_text: compose_data.body.clone(),
            body_html: String::new(), // TODO: Support HTML composition
            attachments: Vec::new(), // TODO: Support draft attachments
            in_reply_to: None, // TODO: Extract from compose data if this is a reply
            references: Vec::new(), // TODO: Extract references for replies
            created_at: now,
            updated_at: now,
            auto_saved,
        }
    }
    
    /// Update draft with new compose data
    pub fn update_from_compose_data(&mut self, compose_data: &EmailComposeData, auto_saved: bool) {
        self.subject = compose_data.subject.clone();
        self.to_addrs = EmailComposeData::parse_addresses(&compose_data.to);
        self.cc_addrs = EmailComposeData::parse_addresses(&compose_data.cc);
        self.bcc_addrs = EmailComposeData::parse_addresses(&compose_data.bcc);
        self.body_text = compose_data.body.clone();
        self.updated_at = Utc::now();
        self.auto_saved = auto_saved;
    }
    
    /// Convert to EmailComposeData for loading in the UI
    pub fn to_compose_data(&self) -> EmailComposeData {
        EmailComposeData {
            to: self.to_addrs.join(", "),
            cc: self.cc_addrs.join(", "),
            bcc: self.bcc_addrs.join(", "),
            subject: self.subject.clone(),
            body: self.body_text.clone(),
        }
    }
}

/// Convert IMAP message to stored message
impl StoredMessage {
    pub fn from_imap_message(
        imap_message: &ImapMessage,
        account_id: String,
        folder_name: String,
    ) -> Self {
        let now = Utc::now();
        
        // Extract envelope information if available
        let envelope = imap_message.envelope.as_ref();
        
        // Generate deterministic ID based on account, folder, and UID to prevent duplicates
        let deterministic_id = {
            let uid = imap_message.uid.unwrap_or(0);
            let id_string = format!("{}:{}:{}", account_id, folder_name, uid);
            // Use a deterministic UUID based on the combination
            let namespace = Uuid::parse_str("6ba7b810-9dad-11d1-80b4-00c04fd430c8").unwrap();
            Uuid::new_v5(&namespace, id_string.as_bytes())
        };
        
        Self {
            id: deterministic_id,
            account_id,
            folder_name,
            imap_uid: imap_message.uid.unwrap_or(0),
            message_id: envelope.and_then(|env| env.message_id.clone()),
            thread_id: None, // Will be computed by threading engine
            in_reply_to: envelope.and_then(|env| env.in_reply_to.clone()),
            references: Vec::new(), // Would need to parse from headers
            subject: envelope
                .and_then(|env| env.subject.clone())
                .unwrap_or_default(),
            from_addr: envelope
                .and_then(|env| env.from.first())
                .and_then(|addr| addr.email_address())
                .unwrap_or_default(),
            from_name: envelope
                .and_then(|env| env.from.first())
                .and_then(|addr| addr.name.clone()),
            to_addrs: envelope
                .map(|env| env.to.iter().filter_map(|addr| addr.email_address()).collect())
                .unwrap_or_default(),
            cc_addrs: envelope
                .map(|env| env.cc.iter().filter_map(|addr| addr.email_address()).collect())
                .unwrap_or_default(),
            bcc_addrs: envelope
                .map(|env| env.bcc.iter().filter_map(|addr| addr.email_address()).collect())
                .unwrap_or_default(),
            reply_to: envelope
                .and_then(|env| env.reply_to.first())
                .and_then(|addr| addr.email_address()),
            date: imap_message.internal_date.unwrap_or(now),
            body_text: Self::parse_and_clean_body_text(&imap_message.body),
            body_html: Self::parse_and_clean_body_html(&imap_message.body),
            attachments: Vec::new(), // Would need body structure parsing
            flags: imap_message.flags.iter().map(|flag| {
                match flag {
                    MessageFlag::Seen => "\\Seen".to_string(),
                    MessageFlag::Answered => "\\Answered".to_string(),
                    MessageFlag::Flagged => "\\Flagged".to_string(),
                    MessageFlag::Deleted => "\\Deleted".to_string(),
                    MessageFlag::Draft => "\\Draft".to_string(),
                    MessageFlag::Recent => "\\Recent".to_string(),
                    MessageFlag::Custom(s) => s.clone(),
                }
            }).collect(),
            labels: Vec::new(), // Gmail-specific labels handled separately
            size: imap_message.size,
            priority: None, // Extract from headers if needed
            created_at: now,
            updated_at: now,
            last_synced: now,
            sync_version: 1,
            is_draft: imap_message.flags.contains(&MessageFlag::Draft),
            is_deleted: imap_message.flags.contains(&MessageFlag::Deleted),
        }
    }
    
    /// Parse and clean body text from raw IMAP content
    fn parse_and_clean_body_text(raw_body: &Option<String>) -> Option<String> {
        let raw_content = match raw_body {
            Some(body) => body,
            None => return None,
        };
        
        tracing::debug!("Parsing body text, raw content length: {}", raw_content.len());
        
        // Clean the raw email content to remove headers and technical data
        let cleaned_content = Self::clean_raw_email_content(raw_content);
        
        // Check if we have HTML content that needs conversion to plain text
        // Use more aggressive HTML detection since is_html_content might miss some cases
        let has_html_tags = cleaned_content.contains("<html") || 
                           cleaned_content.contains("<!DOCTYPE") ||
                           cleaned_content.contains("<div") || 
                           cleaned_content.contains("<span") || 
                           cleaned_content.contains("<p>") || 
                           cleaned_content.contains("<br") ||
                           cleaned_content.contains("<td") || 
                           cleaned_content.contains("<tr") || 
                           cleaned_content.contains("<table") || 
                           cleaned_content.contains("<li") ||
                           cleaned_content.contains("<body") ||
                           cleaned_content.contains("<head");
        
        if has_html_tags || crate::html::is_html_content(&cleaned_content) {
            tracing::info!("Converting HTML content to plain text for body_text (length: {})", cleaned_content.len());
            let html_renderer = crate::html::HtmlRenderer::new(80);
            let plain_text = html_renderer.html_to_plain_text(&cleaned_content);
            
            if !plain_text.trim().is_empty() && plain_text.len() > 10 {
                tracing::info!("HTML-to-text conversion successful: {} chars -> {} chars", cleaned_content.len(), plain_text.len());
                Some(plain_text)
            } else {
                tracing::warn!("HTML-to-text conversion failed, using aggressive text extraction");
                // If HTML conversion fails, try to extract readable text manually
                let manual_text = Self::extract_text_from_html(&cleaned_content);
                if !manual_text.trim().is_empty() {
                    Some(manual_text)
                } else {
                    Some(cleaned_content) // Ultimate fallback
                }
            }
        } else {
            tracing::debug!("Using cleaned plain text content");
            if !cleaned_content.trim().is_empty() {
                Some(cleaned_content)
            } else {
                None
            }
        }
    }
    
    /// Parse and clean HTML body from raw IMAP content
    fn parse_and_clean_body_html(raw_body: &Option<String>) -> Option<String> {
        let raw_content = match raw_body {
            Some(body) => body,
            None => return None,
        };
        
        tracing::debug!("Parsing HTML body, raw content length: {}", raw_content.len());
        
        // Clean the raw email content to remove headers and technical data
        let cleaned_content = Self::clean_raw_email_content(raw_content);
        
        // Only return HTML if it's actually HTML content
        if crate::html::is_html_content(&cleaned_content) {
            tracing::debug!("Found HTML content, storing as HTML body");
            Some(cleaned_content)
        } else {
            tracing::debug!("No HTML content found");
            None
        }
    }
    
    /// Clean raw email content by removing technical headers and metadata
    fn clean_raw_email_content(raw_content: &str) -> String {
        tracing::debug!("Cleaning raw email content of length: {}", raw_content.len());
        
        // Use the more reliable aggressive extraction method as the primary approach
        // This removes headers and technical content, but preserves HTML structure
        // HTML-to-text conversion should happen at higher levels when appropriate
        return Self::aggressive_content_extraction(raw_content);
    }
    
    /// Extract readable text from HTML content by manually removing tags
    fn extract_text_from_html(html_content: &str) -> String {
        tracing::debug!("Manually extracting text from HTML content");
        
        let result = html_content
            .lines()
            .map(|line| {
                let mut text_line = line.to_string();
                
                // Remove HTML tags (simple regex-like approach)
                while text_line.contains('<') && text_line.contains('>') {
                    if let Some(start) = text_line.find('<') {
                        if let Some(end) = text_line[start..].find('>') {
                            let tag_end = start + end + 1;
                            text_line.replace_range(start..tag_end, " ");
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }
                
                // Clean up multiple spaces and trim
                text_line.split_whitespace().collect::<Vec<_>>().join(" ").trim().to_string()
            })
            .filter(|line| !line.is_empty() && line.len() > 2) // Filter very short lines
            .collect::<Vec<_>>()
            .join("\n");
        
        tracing::debug!("Manual HTML text extraction complete: {} chars", result.len());
        result
    }
    
    /// More aggressive content extraction for difficult cases
    fn aggressive_content_extraction(raw_content: &str) -> String {
        let lines: Vec<&str> = raw_content.lines().collect();
        let mut result_lines: Vec<&str> = Vec::new();
        let mut found_content_start = false;
        
        // Comprehensive list of email header patterns to skip
        let header_patterns = [
            // Basic RFC headers (CRITICAL - these show in message content area)
            "from:", "to:", "cc:", "bcc:", "subject:", "date:", "reply-to:", "sender:",
            "message-id:", "in-reply-to:", "references:", "mime-version:",
            // Content and encoding headers
            "content-type:", "content-transfer-encoding:", "content-disposition:",
            "content-id:", "content-description:", "content-language:", "content-length:",
            // Routing and delivery headers (CRITICAL - these are in the screenshots)
            "delivered-to:", "received:", "return-path:", "envelope-to:",
            // Authentication headers (CRITICAL - showing in screenshots)
            "authentication-results:", "received-spf:", "dkim-signature:", "dkim-filter:",
            "arc-seal:", "arc-message-signature:", "arc-authentication-results:",
            // Google/Gmail specific headers
            "x-received:", "x-google-smtp-source:", "x-gm-message-state:", "x-google-dkim-signature:",
            "x-gm-thd-id:", "x-gmail-labels:", "x-google-", "x-gm-",
            // Microsoft/Outlook headers
            "x-ms-exchange-", "x-originating-ip:", "x-microsoft-", "x-ms-",
            // Apple Mail headers (seen in screenshots)
            "x-mailer:", "x-apple-", "--apple-mail", "apple-mail",
            // Other client headers
            "x-kmail-", "x-kde-", "x-evolution-", "x-thunderbird-",
            // Security and spam headers
            "x-spam-", "x-virus-", "x-barracuda-", "x-report-abuse:",
            // Mailing list headers
            "list-id:", "list-unsubscribe:", "list-archive:", "list-post:",
            "list-help:", "list-subscribe:", "precedence:", "feedback-id:",
            // Priority and tracking headers
            "x-priority:", "x-sg-eid:", "importance:", "user-agent:",
            "thread-topic:", "thread-index:", "x-original-to:",
            // Additional patterns that appear in raw emails
            "spf=pass", "dkim=pass", "dmarc=pass", "smtp.mailfrom=", "smtp.helo=",
        ];
        
        for (i, line) in lines.iter().enumerate() {
            let line_lower = line.to_lowercase();
            let line_trimmed = line.trim();
            
            // Skip completely empty lines until we find content
            if line_trimmed.is_empty() && !found_content_start {
                continue;  
            }
            
            // Skip lines that match header patterns (case-insensitive)
            let is_header = header_patterns.iter().any(|&pattern| {
                line_lower.starts_with(pattern) || 
                line_lower.contains(&format!(" {}", pattern)) // Headers can be indented
            });
            
            // Additional header detection - lines that look like email headers
            // Format: "Header Name: Value" at the beginning of content
            let looks_like_header = if !found_content_start && line_trimmed.contains(':') {
                let parts: Vec<&str> = line_trimmed.splitn(2, ':').collect();
                if parts.len() == 2 {
                    let header_name = parts[0].trim().to_lowercase();
                    // Check if it looks like a standard email header
                    header_name == "from" || 
                    header_name == "to" || 
                    header_name == "cc" || 
                    header_name == "bcc" ||
                    header_name == "subject" || 
                    header_name == "date" || 
                    header_name == "reply-to" ||
                    header_name.starts_with("x-") ||
                    header_name.contains("received") ||
                    header_name.contains("delivered") ||
                    header_name.contains("message-id") ||
                    header_name.contains("content-")
                } else {
                    false
                }
            } else {
                false
            };
            
            if is_header || looks_like_header {
                tracing::debug!("Skipping header line {}: {}", i, &line[..std::cmp::min(60, line.len())]);
                continue;
            }
            
            // Skip boundary markers and multipart delimiters
            if line.starts_with("--") && (
                line.contains("Apple-Mail") || 
                line.contains("boundary") || 
                line.contains("=_") ||
                line.len() > 15  // Long boundary markers
            ) {
                tracing::debug!("Skipping boundary line {}: {}", i, &line[..std::cmp::min(40, line.len())]);
                continue;
            }
            
            // Skip lines with technical content patterns
            if line_lower.contains("content-type:") ||
               line_lower.contains("content-transfer-encoding:") ||
               line_lower.contains("charset=") ||
               line_lower.contains("boundary=") ||
               line_lower.contains("multipart/") ||
               line_lower.contains("quoted-printable") ||
               line_lower.contains("base64") ||
               line_lower.contains("mime-version:") ||
               line_lower.contains("us-ascii") ||
               line_lower.contains("utf-8") ||
               line_lower.contains("iso-8859") {
                tracing::debug!("Skipping technical line {}: {}", i, &line[..std::cmp::min(40, line.len())]);
                continue;
            }
            
            // Skip encoded strings (=?charset?encoding?data?=)
            if line.contains("=?") && line.contains("?=") {
                tracing::debug!("Skipping encoded line {}: {}", i, &line[..std::cmp::min(40, line.len())]);
                continue;
            }
            
            // Skip lines that start with = and look like encoded content
            if line_trimmed.starts_with("=") && (
                line_trimmed.contains("ascii") ||
                line_trimmed.contains("Q?") ||
                line_trimmed.contains("B?") ||
                line.len() > 30
            ) {
                tracing::debug!("Skipping encoded content line {}: {}", i, &line[..std::cmp::min(40, line.len())]);
                continue;
            }
            
            // Skip lines that are mostly technical data (base64-like, long strings without spaces)
            if line.len() > 100 && !line.contains(" ") && !line.starts_with("http") {
                let non_alnum_count = line.chars().filter(|c| !c.is_alphanumeric() && *c != '/' && *c != '+' && *c != '=').count();
                if non_alnum_count < 5 { // Looks like encoded data
                    tracing::debug!("Skipping encoded data line {}: {}", i, &line[..std::cmp::min(40, line.len())]);
                    continue;
                }
            }
            
            // Skip lines with excessive technical characters
            if line.len() > 50 {
                let technical_chars = line.chars().filter(|&c| c == '=' || c == ';' || c == ':').count();
                if technical_chars > line.len() / 10 { // More than 10% technical characters
                    tracing::debug!("Skipping technical data line {}: {}", i, &line[..std::cmp::min(40, line.len())]);
                    continue;
                }
            }
            
            // If we reach here, this line looks like actual content
            found_content_start = true;
            result_lines.push(*line);
            
            if result_lines.len() == 1 {
                tracing::debug!("Found first content line at {}: {}", i, &line[..std::cmp::min(50, line.len())]);
            }
        }
        
        let result = result_lines.join("\n").trim().to_string();
        
        tracing::debug!("Aggressive extraction result: {} lines, {} chars (from {} input chars)", 
                       result_lines.len(), result.len(), raw_content.len());
        
        if result.is_empty() {
            "Email content could not be displayed properly.".to_string()
        } else {
            result
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[tokio::test]
    async fn test_database_creation() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db_path_str = db_path.to_str().unwrap();
        
        let db = EmailDatabase::new(db_path_str).await.unwrap();
        assert!(std::path::Path::new(db_path_str).exists());
        
        let stats = db.get_stats().await.unwrap();
        assert_eq!(stats.message_count, 0);
        assert_eq!(stats.account_count, 0);
    }
    
    #[tokio::test]
    async fn test_message_storage_and_retrieval() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db_path_str = db_path.to_str().unwrap();
        
        let db = EmailDatabase::new(db_path_str).await.unwrap();
        
        // Insert a test account first (to satisfy foreign key constraints)
        sqlx::query("INSERT INTO accounts (id, name, email, provider, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)")
            .bind("test-account")
            .bind("Test Account")
            .bind("test@example.com")
            .bind("test")
            .bind(chrono::Utc::now().to_rfc3339())
            .bind(chrono::Utc::now().to_rfc3339())
            .execute(&db.pool)
            .await.unwrap();
            
        // Insert a test folder
        sqlx::query("INSERT INTO folders (account_id, name, full_name, delimiter, attributes, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?)")
            .bind("test-account")
            .bind("INBOX")
            .bind("INBOX")
            .bind(".")
            .bind("[]")
            .bind(chrono::Utc::now().to_rfc3339())
            .bind(chrono::Utc::now().to_rfc3339())
            .execute(&db.pool)
            .await.unwrap();
        
        let message = StoredMessage {
            id: Uuid::new_v4(),
            account_id: "test-account".to_string(),
            folder_name: "INBOX".to_string(),
            imap_uid: 1,
            message_id: Some("test@example.com".to_string()),
            thread_id: None,
            in_reply_to: None,
            references: vec![],
            subject: "Test Subject".to_string(),
            from_addr: "sender@example.com".to_string(),
            from_name: Some("Test Sender".to_string()),
            to_addrs: vec!["recipient@example.com".to_string()],
            cc_addrs: vec![],
            bcc_addrs: vec![],
            reply_to: None,
            date: Utc::now(),
            body_text: Some("Test body".to_string()),
            body_html: None,
            attachments: vec![],
            flags: vec!["\\Seen".to_string()],
            labels: vec![],
            size: Some(100),
            priority: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_synced: Utc::now(),
            sync_version: 1,
            is_draft: false,
            is_deleted: false,
        };
        
        // Store message
        db.store_message(&message).await.unwrap();
        
        // Retrieve message
        let retrieved = db.get_message_by_uid("test-account", "INBOX", 1).await.unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.subject, "Test Subject");
        assert_eq!(retrieved.from_addr, "sender@example.com");
        
        // Get messages from folder
        let messages = db.get_messages("test-account", "INBOX", None, None).await.unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].subject, "Test Subject");
    }
    
    #[tokio::test]
    async fn test_full_text_search() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db_path_str = db_path.to_str().unwrap();
        
        let db = EmailDatabase::new(db_path_str).await.unwrap();
        
        // Insert a test account first (to satisfy foreign key constraints)
        sqlx::query("INSERT INTO accounts (id, name, email, provider, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)")
            .bind("test-account")
            .bind("Test Account")
            .bind("test@example.com")
            .bind("test")
            .bind(chrono::Utc::now().to_rfc3339())
            .bind(chrono::Utc::now().to_rfc3339())
            .execute(&db.pool)
            .await.unwrap();
            
        // Insert a test folder
        sqlx::query("INSERT INTO folders (account_id, name, full_name, delimiter, attributes, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?)")
            .bind("test-account")
            .bind("INBOX")
            .bind("INBOX")
            .bind(".")
            .bind("[]")
            .bind(chrono::Utc::now().to_rfc3339())
            .bind(chrono::Utc::now().to_rfc3339())
            .execute(&db.pool)
            .await.unwrap();
        
        let message = StoredMessage {
            id: Uuid::new_v4(),
            account_id: "test-account".to_string(),
            folder_name: "INBOX".to_string(),
            imap_uid: 1,
            message_id: Some("search-test@example.com".to_string()),
            thread_id: None,
            in_reply_to: None,
            references: vec![],
            subject: "Important Meeting Tomorrow".to_string(),
            from_addr: "boss@company.com".to_string(),
            from_name: Some("The Boss".to_string()),
            to_addrs: vec!["employee@company.com".to_string()],
            cc_addrs: vec![],
            bcc_addrs: vec![],
            reply_to: None,
            date: Utc::now(),
            body_text: Some("Please attend the quarterly meeting tomorrow at 2 PM".to_string()),
            body_html: None,
            attachments: vec![],
            flags: vec![],
            labels: vec![],
            size: Some(200),
            priority: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_synced: Utc::now(),
            sync_version: 1,
            is_draft: false,
            is_deleted: false,
        };
        
        db.store_message(&message).await.unwrap();
        
        // Search by subject
        let results = db.search_messages("test-account", "meeting", None).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].subject, "Important Meeting Tomorrow");
        
        // Search by body
        let results = db.search_messages("test-account", "quarterly", None).await.unwrap();
        assert_eq!(results.len(), 1);
        
        // Search by sender
        let results = db.search_messages("test-account", "boss", None).await.unwrap();
        assert_eq!(results.len(), 1);
    }
}