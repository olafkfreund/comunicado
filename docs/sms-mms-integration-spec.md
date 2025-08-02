# SMS/MMS Integration Technical Specification

> **Version**: 1.0.0  
> **Created**: August 2025  
> **Status**: Draft - Ready for Implementation  
> **Target Integration**: KDE Connect D-Bus Interface

## 1. Overview

This specification defines the technical architecture for integrating SMS/MMS messaging and mobile notification forwarding into Comunicado through KDE Connect's D-Bus interfaces. The integration will provide seamless mobile communication within the terminal email client while maintaining the keyboard-driven workflow that Comunicado users expect.

### 1.1 Objectives

- **Unified Communication Hub**: Integrate SMS/MMS with email and calendar in one TUI interface
- **Real-time Message Sync**: Bidirectional SMS/MMS synchronization with mobile devices
- **Notification Forwarding**: Display mobile notifications within Comunicado's interface
- **Non-blocking Operations**: All mobile communication operations must be asynchronous
- **Cross-platform Support**: Android primary, iOS secondary (with limitations)
- **Privacy-First Design**: All communication through local network only, no cloud services

### 1.2 Integration Benefits

- **Reduced Context Switching**: Handle all communication from one terminal interface
- **Keyboard Efficiency**: Reply to SMS/MMS using Comunicado's efficient text input
- **Message Threading**: Apply email threading concepts to SMS conversations
- **Unified Search**: Search across email, calendar, and SMS/MMS content
- **Notification Management**: Centralized notification handling with persistence

## 2. Architecture Overview

### 2.1 System Components

```
Comunicado TUI Application
├── SMS/MMS Module
│   ├── KDEConnectClient (D-Bus interface)
│   ├── MessageStore (local SMS storage)
│   ├── ConversationManager (threading & organization)
│   └── NotificationForwarder (mobile notifications)
├── UI Integration
│   ├── SMS View (conversation interface)
│   ├── Notification Panel (mobile alerts)
│   └── Unified Search (email + SMS)
└── Background Services
    ├── MessageSyncService (real-time sync)
    ├── NotificationListener (mobile event monitoring)
    └── DeviceDiscovery (KDE Connect device management)
```

### 2.2 External Dependencies

```
KDE Connect Ecosystem
├── kdeconnect-kde (desktop daemon)
├── KDE Connect Android App
├── KDE Connect iOS App (limited)
└── D-Bus System Bus
```

### 2.3 Data Flow

```
Mobile Device → KDE Connect App → Desktop Daemon → D-Bus → Comunicado
                                                            ↓
Local Storage ← Message Processing ← SMS/MMS Module ←────────┘
```

## 3. D-Bus Interface Integration

### 3.1 Primary D-Bus Interfaces

**Device Discovery Interface:**
```rust
Service: org.kde.kdeconnect
Path: /modules/kdeconnect
Interface: org.kde.kdeconnect.daemon

Methods:
- deviceNames() → QStringList
- devices(bool onlyReachable) → QStringList
- deviceIdByName(QString name) → QString
```

**SMS Conversation Interface:**
```rust
Service: org.kde.kdeconnect
Path: /modules/kdeconnect/devices/{deviceId}
Interface: org.kde.kdeconnect.device.conversations

Methods:
- requestAllConversations() → void
- requestConversation(QString threadId) → void
- requestConversationUpdate(QString threadId) → void
- sendMessage(QString message, QStringList addresses) → void

Signals:
- conversationCreated(QVariantMap conversation)
- conversationUpdated(QVariantMap conversation)
- conversationMessageReceived(QVariantMap message)
```

**Notification Interface:**
```rust
Service: org.kde.kdeconnect
Path: /modules/kdeconnect/devices/{deviceId}
Interface: org.kde.kdeconnect.device.notifications

Methods:
- activeNotifications() → QStringList
- sendReply(QString replyId, QString message) → void
- sendAction(QString notificationId, QString action) → void

Signals:
- notificationPosted(QString id, QVariantMap notification)
- notificationRemoved(QString id)
- notificationUpdated(QString id, QVariantMap notification)
- allNotificationsRemoved()
```

### 3.2 Message Data Structures

**SMS Message Format:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmsMessage {
    pub id: i32,                        // Unique message ID
    pub body: String,                   // Message content
    pub addresses: Vec<String>,         // Phone numbers
    pub date: i64,                      // Unix timestamp
    pub message_type: MessageType,      // SMS/MMS indicator
    pub read: bool,                     // Read status
    pub thread_id: i64,                 // Conversation thread
    pub sub_id: i64,                    // Subscription ID
    pub attachments: Vec<Attachment>,   // MMS attachments
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageType {
    Sms = 1,
    Mms = 2,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub part_id: i64,
    pub mime_type: String,
    pub filename: Option<String>,
    pub data: Vec<u8>,                  // Base64 encoded data
}
```

**Conversation Format:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmsConversation {
    pub thread_id: i64,
    pub display_name: String,
    pub addresses: Vec<ContactInfo>,
    pub message_count: i32,
    pub unread_count: i32,
    pub last_message: Option<SmsMessage>,
    pub last_activity: i64,
    pub archived: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactInfo {
    pub address: String,                // Phone number
    pub display_name: Option<String>,   // Contact name if available
}
```

**Mobile Notification Format:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileNotification {
    pub id: String,
    pub app_name: String,
    pub title: String,
    pub text: String,
    pub icon: Option<String>,
    pub time: i64,
    pub dismissable: bool,
    pub has_reply_action: bool,
    pub reply_id: Option<String>,
    pub actions: Vec<NotificationAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationAction {
    pub key: String,
    pub display_name: String,
}
```

## 4. Implementation Modules

### 4.1 KDE Connect D-Bus Client

**File**: `src/mobile/kde_connect_client.rs`

```rust
use dbus::blocking::{Connection, Proxy};
use std::time::Duration;

pub struct KdeConnectClient {
    connection: Connection,
    device_id: Option<String>,
}

impl KdeConnectClient {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let connection = Connection::new_session()?;
        Ok(Self {
            connection,
            device_id: None,
        })
    }

    pub async fn discover_devices(&self) -> Result<Vec<String>, Box<dyn Error>> {
        // Query available KDE Connect devices
    }

    pub async fn connect_device(&mut self, device_id: String) -> Result<(), Box<dyn Error>> {
        // Establish connection to specific device
    }

    pub async fn request_conversations(&self) -> Result<(), Box<dyn Error>> {
        // Request all SMS conversations
    }

    pub async fn send_sms(&self, message: &str, addresses: &[String]) -> Result<(), Box<dyn Error>> {
        // Send SMS message through KDE Connect
    }

    pub async fn listen_for_messages(&self) -> Result<tokio::sync::mpsc::Receiver<SmsMessage>, Box<dyn Error>> {
        // Set up D-Bus signal monitoring for incoming messages
    }

    pub async fn listen_for_notifications(&self) -> Result<tokio::sync::mpsc::Receiver<MobileNotification>, Box<dyn Error>> {
        // Monitor mobile notifications
    }
}
```

### 4.2 Message Storage and Management

**File**: `src/mobile/message_store.rs`

```rust
use sqlx::{SqlitePool, Row};

pub struct MessageStore {
    pool: SqlitePool,
}

impl MessageStore {
    pub async fn new(database_path: &str) -> Result<Self, sqlx::Error> {
        // Initialize SQLite database for SMS storage
    }

    pub async fn store_message(&self, message: &SmsMessage) -> Result<(), sqlx::Error> {
        // Store SMS message with deduplication
    }

    pub async fn get_conversation(&self, thread_id: i64) -> Result<SmsConversation, sqlx::Error> {
        // Retrieve conversation with all messages
    }

    pub async fn get_conversations(&self) -> Result<Vec<SmsConversation>, sqlx::Error> {
        // Get all conversations ordered by last activity
    }

    pub async fn search_messages(&self, query: &str) -> Result<Vec<SmsMessage>, sqlx::Error> {
        // Full-text search across SMS content
    }

    pub async fn mark_as_read(&self, thread_id: i64) -> Result<(), sqlx::Error> {
        // Mark conversation as read
    }

    pub async fn archive_conversation(&self, thread_id: i64) -> Result<(), sqlx::Error> {
        // Archive conversation
    }
}
```

**Database Schema:**
```sql
-- SMS conversations table
CREATE TABLE sms_conversations (
    thread_id INTEGER PRIMARY KEY,
    display_name TEXT NOT NULL,
    last_activity INTEGER NOT NULL,
    message_count INTEGER DEFAULT 0,
    unread_count INTEGER DEFAULT 0,
    archived BOOLEAN DEFAULT FALSE,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

-- SMS messages table
CREATE TABLE sms_messages (
    id INTEGER PRIMARY KEY,
    thread_id INTEGER NOT NULL,
    body TEXT NOT NULL,
    sender_address TEXT NOT NULL,
    recipient_addresses TEXT NOT NULL, -- JSON array
    date INTEGER NOT NULL,
    message_type INTEGER NOT NULL,    -- 1=SMS, 2=MMS
    read BOOLEAN DEFAULT FALSE,
    sub_id INTEGER,
    attachments TEXT,                 -- JSON array
    FOREIGN KEY (thread_id) REFERENCES sms_conversations (thread_id)
);

-- Contact information table
CREATE TABLE sms_contacts (
    address TEXT PRIMARY KEY,
    display_name TEXT,
    last_seen INTEGER NOT NULL,
    message_count INTEGER DEFAULT 0
);

-- Mobile notifications table
CREATE TABLE mobile_notifications (
    id TEXT PRIMARY KEY,
    device_id TEXT NOT NULL,
    app_name TEXT NOT NULL,
    title TEXT NOT NULL,
    text TEXT NOT NULL,
    icon TEXT,
    time INTEGER NOT NULL,
    dismissable BOOLEAN DEFAULT TRUE,
    has_reply_action BOOLEAN DEFAULT FALSE,
    reply_id TEXT,
    actions TEXT,                     -- JSON array
    dismissed BOOLEAN DEFAULT FALSE,
    created_at INTEGER NOT NULL
);

-- Create indexes for performance
CREATE INDEX idx_sms_messages_thread_date ON sms_messages(thread_id, date DESC);
CREATE INDEX idx_sms_messages_body_fts ON sms_messages(body);
CREATE INDEX idx_sms_conversations_activity ON sms_conversations(last_activity DESC);
CREATE INDEX idx_mobile_notifications_time ON mobile_notifications(time DESC);
```

### 4.3 Background Sync Service

**File**: `src/mobile/sync_service.rs`

```rust
use tokio::sync::{mpsc, RwLock};
use std::sync::Arc;

pub struct MobileSyncService {
    kde_connect: Arc<RwLock<KdeConnectClient>>,
    message_store: Arc<MessageStore>,
    notification_manager: Arc<NotificationManager>,
    sync_config: MobileSyncConfig,
}

#[derive(Debug, Clone)]
pub struct MobileSyncConfig {
    pub enabled: bool,
    pub sync_interval_seconds: u64,
    pub auto_mark_read: bool,
    pub forward_notifications: bool,
    pub notification_apps_filter: Vec<String>,
    pub sync_conversations_limit: usize,
}

impl MobileSyncService {
    pub fn new(
        kde_connect: KdeConnectClient,
        message_store: MessageStore,
        notification_manager: NotificationManager,
    ) -> Self {
        // Initialize sync service
    }

    pub async fn start_sync(&self) -> Result<(), Box<dyn Error>> {
        // Start background sync loops
        tokio::spawn(self.clone().message_sync_loop());
        tokio::spawn(self.clone().notification_sync_loop());
        Ok(())
    }

    async fn message_sync_loop(self) -> Result<(), Box<dyn Error>> {
        // Continuous SMS message synchronization
        let mut interval = tokio::time::interval(
            Duration::from_secs(self.sync_config.sync_interval_seconds)
        );

        loop {
            interval.tick().await;
            if let Err(e) = self.sync_messages().await {
                tracing::error!("Message sync failed: {}", e);
            }
        }
    }

    async fn notification_sync_loop(self) -> Result<(), Box<dyn Error>> {
        // Real-time notification monitoring
        let mut notification_receiver = self.kde_connect
            .read().await
            .listen_for_notifications().await?;

        while let Some(notification) = notification_receiver.recv().await {
            if let Err(e) = self.process_notification(notification).await {
                tracing::error!("Notification processing failed: {}", e);
            }
        }

        Ok(())
    }

    async fn sync_messages(&self) -> Result<(), Box<dyn Error>> {
        // Request fresh conversations from KDE Connect
        self.kde_connect.read().await.request_conversations().await?;
        
        // Process incoming messages
        let mut message_receiver = self.kde_connect
            .read().await
            .listen_for_messages().await?;

        while let Some(message) = message_receiver.recv().await {
            self.message_store.store_message(&message).await?;
            self.notify_new_message(&message).await?;
        }

        Ok(())
    }

    async fn process_notification(&self, notification: MobileNotification) -> Result<(), Box<dyn Error>> {
        // Filter notifications based on configuration
        if !self.should_forward_notification(&notification) {
            return Ok(());
        }

        // Store notification
        self.message_store.store_notification(&notification).await?;

        // Forward to Comunicado's notification system
        self.notification_manager.show_mobile_notification(notification).await?;

        Ok(())
    }

    fn should_forward_notification(&self, notification: &MobileNotification) -> bool {
        // Apply filtering rules
        if !self.sync_config.forward_notifications {
            return false;
        }

        if !self.sync_config.notification_apps_filter.is_empty() {
            return self.sync_config.notification_apps_filter
                .contains(&notification.app_name);
        }

        true
    }
}
```

### 4.4 UI Integration

**File**: `src/ui/sms_ui.rs`

```rust
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

pub struct SmsUi {
    message_store: Arc<MessageStore>,
    selected_conversation: Option<i64>,
    conversations: Vec<SmsConversation>,
    current_conversation: Option<SmsConversation>,
    compose_mode: bool,
    compose_text: String,
    scroll_offset: usize,
}

impl SmsUi {
    pub fn new(message_store: Arc<MessageStore>) -> Self {
        Self {
            message_store,
            selected_conversation: None,
            conversations: Vec::new(),
            current_conversation: None,
            compose_mode: false,
            compose_text: String::new(),
            scroll_offset: 0,
        }
    }

    pub async fn refresh_conversations(&mut self) -> Result<(), Box<dyn Error>> {
        self.conversations = self.message_store.get_conversations().await?;
        Ok(())
    }

    pub fn render(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
            .split(area);

        // Render conversation list
        self.render_conversation_list(f, chunks[0]);

        // Render selected conversation
        if let Some(conversation) = &self.current_conversation {
            self.render_conversation(f, chunks[1], conversation);
        } else {
            self.render_welcome(f, chunks[1]);
        }
    }

    fn render_conversation_list(&self, f: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self.conversations
            .iter()
            .map(|conv| {
                let style = if conv.unread_count > 0 {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                let unread_indicator = if conv.unread_count > 0 {
                    format!(" ({})", conv.unread_count)
                } else {
                    String::new()
                };

                ListItem::new(format!("📱 {}{}", conv.display_name, unread_indicator))
                    .style(style)
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().title("SMS Conversations").borders(Borders::ALL))
            .highlight_style(Style::default().bg(Color::DarkGray));

        f.render_widget(list, area);
    }

    fn render_conversation(&self, f: &mut Frame, area: Rect, conversation: &SmsConversation) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),     // Header
                Constraint::Min(0),        // Messages
                Constraint::Length(3),     // Compose area
            ])
            .split(area);

        // Render conversation header
        let header = Paragraph::new(format!(
            "📱 {} | {} messages | {} unread",
            conversation.display_name,
            conversation.message_count,
            conversation.unread_count
        ))
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::Cyan));

        f.render_widget(header, chunks[0]);

        // Render messages (implementation would fetch and display messages)
        self.render_message_list(f, chunks[1], conversation.thread_id);

        // Render compose area
        if self.compose_mode {
            self.render_compose_area(f, chunks[2]);
        } else {
            let help = Paragraph::new("Press 'c' to compose, 'r' to reply, 'Enter' to open conversation")
                .block(Block::default().borders(Borders::ALL))
                .style(Style::default().fg(Color::Gray));
            f.render_widget(help, chunks[2]);
        }
    }

    fn render_message_list(&self, f: &mut Frame, area: Rect, thread_id: i64) {
        // Implementation would fetch messages for the conversation
        // and render them in a scrollable list format
        let placeholder = Paragraph::new("Loading messages...")
            .block(Block::default().title("Messages").borders(Borders::ALL))
            .wrap(Wrap { trim: true });

        f.render_widget(placeholder, area);
    }

    fn render_compose_area(&self, f: &mut Frame, area: Rect) {
        let compose = Paragraph::new(self.compose_text.as_str())
            .block(Block::default().title("Compose SMS").borders(Borders::ALL))
            .wrap(Wrap { trim: true });

        f.render_widget(compose, area);
    }

    fn render_welcome(&self, f: &mut Frame, area: Rect) {
        let welcome = Paragraph::new(
            "Welcome to SMS/MMS Integration\n\n\
            📱 Select a conversation to view messages\n\
            💬 Press 'c' to start a new conversation\n\
            🔄 Sync is active with your mobile device\n\n\
            Keyboard shortcuts:\n\
            - j/k: Navigate conversations\n\
            - Enter: Open conversation\n\
            - c: Compose new message\n\
            - r: Reply to message\n\
            - /: Search messages"
        )
        .block(Block::default().title("SMS/MMS").borders(Borders::ALL))
        .wrap(Wrap { trim: true });

        f.render_widget(welcome, area);
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> Result<bool, Box<dyn Error>> {
        match key.code {
            KeyCode::Char('c') => {
                self.compose_mode = true;
                self.compose_text.clear();
            }
            KeyCode::Enter if !self.compose_mode => {
                if let Some(conv) = self.conversations.get(self.selected_index) {
                    self.selected_conversation = Some(conv.thread_id);
                    // Load conversation details
                }
            }
            KeyCode::Esc => {
                self.compose_mode = false;
                self.compose_text.clear();
            }
            KeyCode::Char(c) if self.compose_mode => {
                self.compose_text.push(c);
            }
            KeyCode::Backspace if self.compose_mode => {
                self.compose_text.pop();
            }
            _ => return Ok(false),
        }
        Ok(true)
    }
}
```

### 4.5 Notification Integration

**File**: `src/mobile/notification_bridge.rs`

```rust
use crate::notifications::{NotificationManager, NotificationEvent, NotificationPriority};

pub struct MobileNotificationBridge {
    notification_manager: Arc<NotificationManager>,
    message_store: Arc<MessageStore>,
}

impl MobileNotificationBridge {
    pub fn new(
        notification_manager: Arc<NotificationManager>,
        message_store: Arc<MessageStore>,
    ) -> Self {
        Self {
            notification_manager,
            message_store,
        }
    }

    pub async fn handle_mobile_notification(&self, notification: MobileNotification) -> Result<(), Box<dyn Error>> {
        // Convert mobile notification to Comunicado notification format
        let comunicado_event = NotificationEvent::Mobile {
            app_name: notification.app_name.clone(),
            title: notification.title.clone(),
            message: notification.text.clone(),
            time: chrono::DateTime::from_timestamp(notification.time / 1000, 0)
                .unwrap_or_else(|| chrono::Utc::now()),
            priority: self.determine_priority(&notification),
            has_reply: notification.has_reply_action,
            reply_id: notification.reply_id.clone(),
        };

        // Show notification in Comunicado's UI
        self.notification_manager.show_notification(comunicado_event).await?;

        // Store for persistence
        self.message_store.store_notification(&notification).await?;

        Ok(())
    }

    pub async fn handle_sms_message(&self, message: SmsMessage) -> Result<(), Box<dyn Error>> {
        // Create notification for incoming SMS
        let notification_event = NotificationEvent::SMS {
            sender: message.addresses.first().cloned().unwrap_or_default(),
            preview: self.truncate_message(&message.body, 100),
            thread_id: message.thread_id,
            message_id: message.id,
            priority: NotificationPriority::Normal,
        };

        self.notification_manager.show_notification(notification_event).await?;

        Ok(())
    }

    fn determine_priority(&self, notification: &MobileNotification) -> NotificationPriority {
        match notification.app_name.as_str() {
            "Messages" | "SMS" | "WhatsApp" | "Telegram" => NotificationPriority::High,
            "Email" | "Gmail" | "Outlook" => NotificationPriority::Normal,
            "News" | "Social" => NotificationPriority::Low,
            _ => NotificationPriority::Normal,
        }
    }

    fn truncate_message(&self, message: &str, max_length: usize) -> String {
        if message.len() <= max_length {
            message.to_string()
        } else {
            format!("{}...", &message[..max_length])
        }
    }
}
```

## 5. Configuration and Settings

### 5.1 Mobile Integration Settings

**Settings UI Extension** (`src/ui/settings_ui.rs`):

```rust
// Add to General settings tab
"📱 SMS/MMS Integration",
"   🔗 KDE Connect enabled: {}",
"   📲 Auto-sync messages: {}",
"   🔔 Forward notifications: {}",
"   ⏱️  Sync interval: {} seconds",
"   📋 Max conversations: {}",
"   🎯 Notification apps filter: {}",

// Mobile-specific settings tab
"Mobile" => {
    items.extend([
        "📱 Device Management",
        "   🔍 Discovered devices: {}",
        "   ✅ Connected device: {}",
        "   🔐 Pairing status: {}",
        "",
        "💬 Message Settings",
        "   📥 Auto-download MMS: {}",
        "   📤 Mark as read on reply: {}",
        "   🗂️  Archive old conversations: {}",
        "   📊 Message retention days: {}",
        "",
        "🔔 Notification Settings",
        "   📱 Forward all notifications: {}",
        "   🎯 Filtered apps: {}",
        "   🔇 Quiet hours: {} - {}",
        "   💬 Show message preview: {}",
    ]);
}
```

### 5.2 Configuration File Format

**File**: `config/mobile.toml`

```toml
[mobile]
enabled = true
kde_connect_device_id = ""
auto_pair = true

[sms]
enabled = true
sync_interval_seconds = 30
auto_mark_read = false
max_conversations = 100
archive_after_days = 90
download_mms_automatically = true

[notifications]
forward_enabled = true
show_preview = true
filtered_apps = [
    "Messages",
    "WhatsApp", 
    "Telegram",
    "Signal"
]

[notifications.quiet_hours]
enabled = false
start_time = "22:00"
end_time = "08:00"

[storage]
database_path = "data/mobile.db"
backup_enabled = true
backup_interval_hours = 24
max_backup_files = 7
```

## 6. Error Handling and Resilience

### 6.1 Connection Management

```rust
#[derive(Debug)]
pub enum MobileIntegrationError {
    KdeConnectNotAvailable,
    DeviceNotPaired,
    DeviceNotReachable,
    DbusConnectionFailed,
    MessageSendFailed,
    NotificationFailed,
    DatabaseError(sqlx::Error),
    ConfigurationError(String),
}

impl MobileIntegrationError {
    pub fn user_message(&self) -> &'static str {
        match self {
            Self::KdeConnectNotAvailable => "KDE Connect is not installed or running",
            Self::DeviceNotPaired => "Mobile device is not paired with KDE Connect",
            Self::DeviceNotReachable => "Mobile device is not reachable on the network",
            Self::DbusConnectionFailed => "Failed to connect to D-Bus service",
            Self::MessageSendFailed => "Failed to send SMS message",
            Self::NotificationFailed => "Failed to process mobile notification",
            Self::DatabaseError(_) => "Database operation failed",
            Self::ConfigurationError(_) => "Mobile integration configuration error",
        }
    }

    pub fn is_recoverable(&self) -> bool {
        matches!(self, 
            Self::DeviceNotReachable | 
            Self::MessageSendFailed |
            Self::NotificationFailed
        )
    }
}
```

### 6.2 Retry Logic and Graceful Degradation

```rust
pub struct RetryConfig {
    pub max_attempts: u32,
    pub base_delay: Duration,
    pub max_delay: Duration,
    pub exponential_base: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(30),
            exponential_base: 2.0,
        }
    }
}

async fn retry_with_backoff<F, T, E>(
    operation: F,
    config: &RetryConfig,
) -> Result<T, E>
where
    F: Fn() -> futures::future::BoxFuture<'_, Result<T, E>>,
    E: std::fmt::Debug,
{
    for attempt in 0..config.max_attempts {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) if attempt + 1 >= config.max_attempts => return Err(e),
            Err(e) => {
                let delay = std::cmp::min(
                    config.base_delay.mul_f64(config.exponential_base.powi(attempt as i32)),
                    config.max_delay,
                );
                
                tracing::warn!("Attempt {} failed: {:?}, retrying in {:?}", attempt + 1, e, delay);
                tokio::time::sleep(delay).await;
            }
        }
    }
    
    unreachable!()
}
```

## 7. Security and Privacy Considerations

### 7.1 Data Protection

- **Local Storage Only**: All SMS/MMS data stored locally in encrypted SQLite database
- **No Cloud Sync**: No data transmitted to external services beyond local network
- **Encryption at Rest**: Message database encrypted with user-provided key
- **Memory Safety**: Rust's memory safety prevents data leaks
- **Secure Deletion**: Proper cleanup of sensitive data from memory

### 7.2 Network Security

- **TLS Encryption**: All KDE Connect communication uses TLS
- **Local Network Only**: Communication restricted to local network segment
- **Device Authentication**: Mutual authentication between devices required
- **Permission Model**: Explicit user consent for each capability

### 7.3 Privacy Controls

```rust
#[derive(Debug, Clone)]
pub struct PrivacySettings {
    pub log_message_content: bool,        // Log actual message content
    pub store_contact_names: bool,        // Store contact information
    pub notification_preview: bool,       // Show message preview in notifications
    pub analytics_enabled: bool,          // Collect usage analytics
    pub backup_include_content: bool,     // Include message content in backups
}

impl Default for PrivacySettings {
    fn default() -> Self {
        Self {
            log_message_content: false,
            store_contact_names: true,
            notification_preview: true,
            analytics_enabled: false,
            backup_include_content: false,
        }
    }
}
```

## 8. Testing Strategy

### 8.1 Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test;

    #[tokio::test]
    async fn test_sms_message_storage() {
        let store = MessageStore::new(":memory:").await.unwrap();
        
        let message = SmsMessage {
            id: 1,
            body: "Test message".to_string(),
            addresses: vec!["+1234567890".to_string()],
            date: chrono::Utc::now().timestamp(),
            message_type: MessageType::Sms,
            read: false,
            thread_id: 1,
            sub_id: 1,
            attachments: vec![],
        };

        store.store_message(&message).await.unwrap();
        
        let conversation = store.get_conversation(1).await.unwrap();
        assert_eq!(conversation.message_count, 1);
        assert_eq!(conversation.unread_count, 1);
    }

    #[tokio::test]
    async fn test_notification_filtering() {
        let bridge = MobileNotificationBridge::new(
            Arc::new(NotificationManager::new()),
            Arc::new(MessageStore::new(":memory:").await.unwrap()),
        );

        let notification = MobileNotification {
            id: "test".to_string(),
            app_name: "Messages".to_string(),
            title: "New message".to_string(),
            text: "Hello world".to_string(),
            icon: None,
            time: chrono::Utc::now().timestamp(),
            dismissable: true,
            has_reply_action: true,
            reply_id: Some("reply-123".to_string()),
            actions: vec![],
        };

        let priority = bridge.determine_priority(&notification);
        assert_eq!(priority, NotificationPriority::High);
    }
}
```

### 8.2 Integration Tests

```rust
#[tokio::test]
async fn test_kde_connect_integration() {
    // This test requires KDE Connect to be running
    if std::env::var("SKIP_INTEGRATION_TESTS").is_ok() {
        return;
    }

    let client = KdeConnectClient::new().unwrap();
    let devices = client.discover_devices().await.unwrap();
    
    if !devices.is_empty() {
        let device_id = &devices[0];
        client.connect_device(device_id.clone()).await.unwrap();
        
        // Test message listening
        let mut receiver = client.listen_for_messages().await.unwrap();
        
        // Send test message and verify reception
        client.send_sms("Test message from Comunicado", &["+1234567890"]).await.unwrap();
        
        // Note: This test requires actual device interaction
    }
}
```

### 8.3 Mock Testing

```rust
pub struct MockKdeConnectClient {
    conversations: Vec<SmsConversation>,
    notifications: Vec<MobileNotification>,
}

impl MockKdeConnectClient {
    pub fn new() -> Self {
        Self {
            conversations: vec![
                SmsConversation {
                    thread_id: 1,
                    display_name: "John Doe".to_string(),
                    addresses: vec![ContactInfo {
                        address: "+1234567890".to_string(),
                        display_name: Some("John Doe".to_string()),
                    }],
                    message_count: 5,
                    unread_count: 2,
                    last_message: None,
                    last_activity: chrono::Utc::now().timestamp(),
                    archived: false,
                },
            ],
            notifications: vec![],
        }
    }

    pub async fn simulate_incoming_message(&mut self) -> SmsMessage {
        SmsMessage {
            id: 42,
            body: "Hello from mock device!".to_string(),
            addresses: vec!["+1234567890".to_string()],
            date: chrono::Utc::now().timestamp(),
            message_type: MessageType::Sms,
            read: false,
            thread_id: 1,
            sub_id: 1,
            attachments: vec![],
        }
    }
}
```

## 9. Performance Considerations

### 9.1 Optimization Strategies

**Message Sync Optimization:**
- Incremental sync: Only fetch messages newer than last sync timestamp
- Batch processing: Group multiple messages in single database transaction
- Connection pooling: Reuse D-Bus connections for multiple operations
- Background processing: All sync operations non-blocking

**UI Performance:**
- Virtual scrolling: Only render visible messages in conversation view
- Message pagination: Load messages in chunks of 50-100
- Lazy loading: Load conversation details only when selected
- Debounced search: Delay search execution to avoid excessive queries

**Memory Management:**
- Message limits: Keep only recent N messages in memory
- Attachment handling: Stream large attachments instead of loading entirely
- Cleanup: Regular cleanup of old notifications and temporary data
- Compression: Compress stored attachments and media

### 9.2 Resource Monitoring

```rust
#[derive(Debug, Clone)]
pub struct MobileIntegrationStats {
    pub connected_devices: usize,
    pub active_conversations: usize,
    pub messages_stored: u64,
    pub notifications_received: u64,
    pub sync_operations: u64,
    pub last_sync_duration: Option<Duration>,
    pub database_size_bytes: u64,
    pub memory_usage_bytes: u64,
}

impl MobileSyncService {
    pub async fn get_stats(&self) -> MobileIntegrationStats {
        MobileIntegrationStats {
            connected_devices: self.get_connected_device_count().await,
            active_conversations: self.message_store.get_conversation_count().await.unwrap_or(0),
            messages_stored: self.message_store.get_message_count().await.unwrap_or(0),
            notifications_received: self.notification_count.load(Ordering::Relaxed),
            sync_operations: self.sync_count.load(Ordering::Relaxed),
            last_sync_duration: self.last_sync_duration.read().await.clone(),
            database_size_bytes: self.get_database_size().await.unwrap_or(0),
            memory_usage_bytes: self.get_memory_usage(),
        }
    }
}
```

## 10. Future Enhancements

### 10.1 Phase 2 Features

**Advanced Messaging:**
- Group SMS/MMS support
- Message reactions and read receipts
- Message scheduling and delayed send
- Rich media preview (images, videos, documents)

**Smart Features:**
- AI-powered message categorization
- Automated response suggestions
- Smart notification filtering
- Contact management integration

**Cross-Platform Expansion:**
- Direct iOS integration (bypassing KDE Connect limitations)
- Android app companion for enhanced features
- Web-based configuration interface
- Bridge to other messaging platforms (Signal, WhatsApp)

### 10.2 Integration Enhancements

**Unified Communication:**
- Message threading across email and SMS
- Unified search across all communication types
- Contact synchronization with email address book
- Calendar integration for message-based scheduling

**Workflow Automation:**
- Rules-based message processing
- Integration with task management
- Email-to-SMS forwarding rules
- Automated backup and archiving

## 11. Implementation Timeline

### Phase 1: Foundation (4-6 weeks)
- ✅ KDE Connect D-Bus client implementation
- ✅ Basic SMS message storage and retrieval
- ✅ Simple conversation UI
- ✅ Device discovery and connection

### Phase 2: Core Features (4-6 weeks)
- ✅ Real-time message synchronization
- ✅ Notification forwarding and integration
- ✅ SMS composition and sending
- ✅ Settings UI integration

### Phase 3: Polish and Enhancement (3-4 weeks)
- ✅ MMS support and attachment handling
- ✅ Advanced UI features (search, threading)
- ✅ Error handling and resilience
- ✅ Comprehensive testing

### Phase 4: Optimization (2-3 weeks)
- ✅ Performance optimization
- ✅ Memory usage optimization
- ✅ Documentation and user guides
- ✅ Production readiness

---

**Total Estimated Timeline:** 13-19 weeks  
**Priority Level:** High - Significant value-add for terminal productivity users  
**Risk Level:** Medium - Dependent on KDE Connect stability and device compatibility  
**Success Metrics:** 
- Seamless SMS/MMS sync with <5 second latency
- Zero UI blocking during mobile operations  
- 99%+ message delivery reliability
- Positive user feedback on unified communication experience

This specification provides a comprehensive technical foundation for implementing SMS/MMS integration into Comunicado while maintaining the application's focus on terminal-native productivity and keyboard-driven workflows.