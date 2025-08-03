# KDE Connect Plugin Integration Guide

> Technical Documentation for SMS/MMS Integration via KDE Connect
> Version: 1.0.0
> Last Updated: 2025-08-02

## Overview

This document provides comprehensive technical documentation for the KDE Connect plugin integration in Comunicado, enabling SMS/MMS functionality through mobile device connectivity.

## Architecture Overview

### System Components

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   Comunicado    │    │   KDE Connect    │    │  Mobile Device  │
│   TUI Client    │◄──►│     D-Bus        │◄──►│   (Android)     │
│                 │    │   Integration    │    │                 │
└─────────────────┘    └──────────────────┘    └─────────────────┘
         │                       │                       │
         ▼                       ▼                       ▼
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│ Mobile Module   │    │  kdeconnect-cli  │    │  SMS Plugin     │
│ - Storage       │    │  Command Line    │    │  - Send SMS     │
│ - Sync Service  │    │  Interface       │    │  - Receive SMS  │
│ - UI Components │    │                  │    │  - Notifications│
└─────────────────┘    └──────────────────┘    └─────────────────┘
```

### Core Technologies

- **KDE Connect Framework**: Cross-platform device connectivity
- **D-Bus Protocol**: Inter-process communication on Linux
- **SQLite Database**: Local message storage and indexing
- **Tokio Runtime**: Async background processing
- **Ratatui**: Terminal user interface

## Installation and Setup

### Prerequisites

1. **KDE Connect Installation**
   ```bash
   # Ubuntu/Debian
   sudo apt install kdeconnect

   # Fedora
   sudo dnf install kdeconnect

   # Arch Linux
   sudo pacman -S kdeconnect
   ```

2. **Mobile App Installation**
   - Install "KDE Connect" from Google Play Store or F-Droid
   - Available for Android devices

3. **Network Configuration**
   - Ensure both devices are on the same WiFi network
   - Firewall must allow KDE Connect ports (1714-1764 TCP/UDP)

### Device Pairing Process

1. **Start KDE Connect Service**
   ```bash
   # Start the KDE Connect daemon
   kdeconnect-cli --refresh

   # List available devices
   kdeconnect-cli --list-available
   ```

2. **Pair Devices**
   ```bash
   # Request pairing with device
   kdeconnect-cli --pair --device [DEVICE_ID]

   # Accept pairing request on mobile device
   # Verify pairing status
   kdeconnect-cli --list-available
   ```

3. **Enable SMS Plugin**
   ```bash
   # Check available plugins
   kdeconnect-cli --list-available --device [DEVICE_ID]

   # The SMS plugin should be automatically enabled
   # Verify SMS functionality
   kdeconnect-cli --device [DEVICE_ID] --send-sms "Test message" --destination "+1234567890"
   ```

## Integration Architecture

### Mobile Module Structure

```
src/mobile/
├── mod.rs                 # Module configuration and exports
├── config.rs             # Configuration management
├── storage.rs            # SQLite database layer
├── services.rs           # Background sync service
├── ui.rs                 # TUI interface components
└── kde_connect/
    ├── mod.rs            # KDE Connect module exports
    ├── simple_client.rs  # Production KDE Connect client
    ├── types.rs          # Data structures and types
    └── utils.rs          # Utility functions
```

### Database Schema

The SMS/MMS storage uses a normalized SQLite schema:

```sql
-- Conversations table
CREATE TABLE conversations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    thread_id INTEGER NOT NULL UNIQUE,
    display_name TEXT NOT NULL,
    last_message_date INTEGER NOT NULL,
    unread_count INTEGER NOT NULL DEFAULT 0,
    is_archived BOOLEAN NOT NULL DEFAULT FALSE,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    metadata TEXT -- JSON metadata
);

-- Messages table
CREATE TABLE messages (
    id INTEGER PRIMARY KEY,
    conversation_id INTEGER NOT NULL,
    body TEXT NOT NULL,
    message_type TEXT NOT NULL, -- 'sms' or 'mms'
    is_read BOOLEAN NOT NULL DEFAULT FALSE,
    date_sent INTEGER NOT NULL,
    date_received INTEGER NOT NULL,
    sub_id INTEGER,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    FOREIGN KEY (conversation_id) REFERENCES conversations(id)
);

-- Conversation contacts table
CREATE TABLE conversation_contacts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    conversation_id INTEGER NOT NULL,
    phone_number TEXT NOT NULL,
    display_name TEXT,
    is_primary BOOLEAN NOT NULL DEFAULT FALSE,
    created_at INTEGER NOT NULL,
    FOREIGN KEY (conversation_id) REFERENCES conversations(id)
);

-- Message attachments table
CREATE TABLE message_attachments (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    message_id INTEGER NOT NULL,
    filename TEXT NOT NULL,
    mime_type TEXT NOT NULL,
    file_size INTEGER,
    data BLOB,
    is_downloaded BOOLEAN NOT NULL DEFAULT FALSE,
    download_url TEXT,
    created_at INTEGER NOT NULL,
    FOREIGN KEY (message_id) REFERENCES messages(id)
);
```

### Service Architecture

#### Background Sync Service

The `MobileSyncService` provides:

- **Periodic Synchronization**: Configurable intervals (default: 30 seconds)
- **Real-time Message Processing**: Immediate handling of new messages
- **Device Management**: Connection monitoring and reconnection
- **Statistics Tracking**: Performance metrics and usage data

```rust
pub struct MobileSyncService {
    config: Arc<RwLock<MobileConfig>>,
    kde_connect: Arc<Mutex<KdeConnectClient>>,
    message_store: Arc<MessageStore>,
    stats: Arc<RwLock<MobileSyncStats>>,
    // Control channels for service management
    control_tx: mpsc::UnboundedSender<ServiceControl>,
    control_rx: Arc<Mutex<mpsc::UnboundedReceiver<ServiceControl>>>,
    is_running: Arc<RwLock<bool>>,
}
```

#### KDE Connect Client

The production client (`simple_client.rs`) provides:

- **Device Discovery**: Automatic detection of available devices
- **SMS Operations**: Send and receive SMS messages
- **Notification Handling**: Mobile notification forwarding
- **Connection Management**: Robust connection handling with retries

```rust
pub struct KdeConnectClient {
    device_id: Option<String>,
    timeout: Duration,
    kde_connect_available: bool,
}
```

## API Reference

### Core Methods

#### Message Storage

```rust
// Store a new SMS/MMS message
pub async fn store_message(&self, message: SmsMessage) -> Result<i64>;

// Retrieve conversations with filtering
pub async fn get_conversations(&self, query: &MessageQuery) -> Result<Vec<SmsConversation>>;

// Get messages for a specific conversation
pub async fn get_messages(&self, conversation_id: i64, query: &MessageQuery) -> Result<Vec<SmsMessage>>;

// Mark message as read
pub async fn mark_message_read(&self, message_id: i64) -> Result<()>;

// Search messages by content
pub async fn search_messages(&self, search_term: &str, limit: Option<usize>) -> Result<Vec<SmsMessage>>;
```

#### Service Control

```rust
// Start the sync service
pub async fn start(&self) -> Result<()>;

// Stop the sync service
pub async fn stop(&self) -> Result<()>;

// Send SMS message
pub async fn send_sms(&self, phone_number: &str, message: &str) -> Result<()>;

// Get service statistics
pub async fn get_stats(&self) -> MobileSyncStats;
```

#### KDE Connect Operations

```rust
// Discover available devices
pub fn discover_devices(&self) -> Result<Vec<DeviceInfo>>;

// Connect to a specific device
pub fn connect_device(&mut self, device_id: String) -> Result<()>;

// Send SMS via KDE Connect
pub fn send_sms(&self, phone_number: &str, message: &str) -> Result<()>;

// Check connection status
pub fn is_connected(&self) -> bool;
```

### Data Structures

#### SmsMessage

```rust
pub struct SmsMessage {
    pub id: i64,                          // Unique message ID
    pub body: String,                     // Message content
    pub addresses: Vec<String>,           // Phone numbers
    pub date: i64,                        // Timestamp (milliseconds)
    pub message_type: MessageType,        // SMS or MMS
    pub read: bool,                       // Read status
    pub thread_id: i64,                   // Conversation thread ID
    pub sub_id: i64,                      // Subscription ID
    pub attachments: Vec<Attachment>,     // MMS attachments
}
```

#### SmsConversation

```rust
pub struct SmsConversation {
    pub id: i64,                          // Database ID
    pub thread_id: i64,                   // Thread identifier
    pub display_name: String,             // Conversation title
    pub participants: Vec<ContactInfo>,   // Conversation participants
    pub message_count: i32,               // Total messages
    pub unread_count: i32,                // Unread messages
    pub last_message_date: DateTime<Utc>, // Last activity
    pub is_archived: bool,                // Archive status
    pub messages: Vec<SmsMessage>,        // Message list
}
```

## Configuration

### Mobile Configuration

```rust
pub struct MobileConfig {
    pub integration_enabled: bool,        // Enable/disable mobile features
    pub sync_interval_seconds: u64,       // Background sync frequency
    pub auto_mark_read: bool,             // Auto-mark as read
    pub notification_filtering: NotificationFilter, // Notification rules
    pub quiet_hours: Option<QuietHours>,  // Do not disturb schedule
    pub preferred_device_id: Option<String>, // Default device
    pub storage_retention_days: i64,      // Message retention
    pub backup_enabled: bool,             // Enable backups
}
```

### Example Configuration

```toml
[mobile]
integration_enabled = true
sync_interval_seconds = 30
auto_mark_read = false
storage_retention_days = 365
backup_enabled = true

[mobile.notification_filtering]
priority_threshold = "normal"
keywords_block = ["spam", "promotion"]
keywords_allow = ["urgent", "important"]

[mobile.quiet_hours]
enabled = true
start_time = "22:00"
end_time = "07:00"
timezone = "UTC"
```

## Error Handling

### Common Error Types

```rust
pub enum MobileError {
    // KDE Connect errors
    KdeConnectNotAvailable(String),
    DeviceNotPaired(String),
    DeviceNotReachable(String),
    
    // Database errors
    DatabaseError(sqlx::Error),
    StorageCorrupted(String),
    
    // Network errors
    NetworkTimeout(String),
    ConnectionFailed(String),
    
    // Message errors
    InvalidPhoneNumber(String),
    MessageTooLong(String),
    
    // Configuration errors
    ConfigurationInvalid(String),
    PermissionDenied(String),
}
```

### Error Recovery Strategies

1. **Connection Failures**: Automatic retry with exponential backoff
2. **Database Corruption**: Automatic schema recreation and data recovery
3. **Device Unavailable**: Graceful degradation with user notification
4. **Permission Issues**: Clear instructions for user resolution

## Performance Optimization

### Database Optimizations

- **Indexes**: Optimized queries with strategic indexes
- **Connection Pooling**: SQLite connection pool for concurrent access
- **WAL Journaling**: Write-Ahead Logging for better performance
- **Batch Operations**: Bulk message processing for sync efficiency

### Memory Management

- **Arc/Mutex**: Thread-safe shared state management
- **Async Processing**: Non-blocking operations with tokio
- **Lazy Loading**: On-demand message and attachment loading
- **Cache Management**: LRU cache for frequently accessed data

### Network Efficiency

- **Delta Sync**: Only sync changed messages
- **Compression**: Message content compression for large transfers
- **Rate Limiting**: Respectful API usage to prevent throttling
- **Connection Reuse**: Persistent connections when possible

## Security Considerations

### Data Protection

- **Local Storage**: All messages stored locally, no cloud sync
- **Encryption**: Optional database encryption for sensitive data
- **Access Control**: Proper file permissions for database files
- **Privacy**: No telemetry or usage tracking

### Network Security

- **Local Network**: Communication limited to local network
- **Device Verification**: Cryptographic device pairing
- **Protocol Security**: KDE Connect's built-in security measures
- **Firewall Rules**: Minimal port exposure requirements

## Troubleshooting

### Common Issues

1. **KDE Connect Not Available**
   ```bash
   # Install KDE Connect
   sudo apt install kdeconnect
   
   # Start the service
   systemctl --user start kdeconnect
   ```

2. **Device Not Found**
   ```bash
   # Refresh device list
   kdeconnect-cli --refresh
   
   # Check network connectivity
   ping [mobile_device_ip]
   ```

3. **Pairing Problems**
   ```bash
   # Unpair and re-pair devices
   kdeconnect-cli --unpair --device [DEVICE_ID]
   kdeconnect-cli --pair --device [DEVICE_ID]
   ```

4. **SMS Not Working**
   ```bash
   # Verify SMS plugin is enabled
   kdeconnect-cli --list-available --device [DEVICE_ID]
   
   # Test SMS functionality
   kdeconnect-cli --device [DEVICE_ID] --send-sms "Test" --destination "+1234567890"
   ```

### Debug Mode

Enable debug logging for troubleshooting:

```bash
# Set environment variable
export RUST_LOG=comunicado::mobile=debug

# Run Comunicado
comunicado
```

### Log Analysis

Key log indicators:

- `KDE Connect availability: true/false` - Service detection
- `Created new conversation` - Message storage
- `Successfully paired with device` - Connection status
- `Sync completed: X messages processed` - Background sync

## Development Guide

### Building from Source

```bash
# Clone repository
git clone https://github.com/user/comunicado
cd comunicado

# Build with mobile features
cargo build --features kde-connect

# Run tests
cargo test mobile
```

### Testing Framework

```bash
# Run all mobile tests
cargo test --package comunicado --lib mobile

# Run specific test categories
cargo test storage  # Database tests
cargo test ui       # Interface tests
cargo test service  # Background service tests
```

### Contributing

1. **Code Style**: Follow existing Rust conventions
2. **Testing**: Maintain 100% test coverage for new features
3. **Documentation**: Update docs for any API changes
4. **Performance**: Profile changes for performance impact

## Future Roadmap

### Planned Features

- **MMS Support**: Full multimedia message support
- **Group Messaging**: Enhanced group conversation handling
- **Message Encryption**: End-to-end encryption for sensitive messages
- **Cross-Platform**: Windows and macOS support via alternative protocols
- **Backup/Restore**: Cloud backup integration options
- **Message Search**: Advanced search with filters and indexing
- **Custom Notifications**: Configurable notification rules
- **Integration APIs**: REST API for third-party integrations

### Version History

- **v1.0.0**: Initial SMS integration with KDE Connect
- **v0.9.0**: Beta testing and bug fixes
- **v0.8.0**: UI implementation and testing
- **v0.7.0**: Background service implementation
- **v0.6.0**: Database layer and storage
- **v0.5.0**: KDE Connect client implementation

## Support and Resources

### Documentation Links

- [KDE Connect Official Docs](https://kdeconnect.kde.org/)
- [D-Bus Specification](https://dbus.freedesktop.org/doc/dbus-specification.html)
- [SQLite Documentation](https://sqlite.org/docs.html)
- [Tokio Async Runtime](https://tokio.rs/)

### Community Support

- GitHub Issues: Report bugs and feature requests
- Discussions: Community support and questions
- Wiki: Additional examples and use cases

### Professional Support

For enterprise deployments and custom integrations, professional support is available through the project maintainers.