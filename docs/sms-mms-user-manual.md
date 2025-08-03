# SMS/MMS User Manual for Comunicado

> Complete User Guide for Mobile Messaging Integration
> Version: 1.0.0
> Last Updated: 2025-08-02

## Table of Contents

1. [Getting Started](#getting-started)
2. [Setup and Configuration](#setup-and-configuration)
3. [Using SMS/MMS Features](#using-smsmms-features)
4. [Interface Guide](#interface-guide)
5. [Advanced Features](#advanced-features)
6. [Troubleshooting](#troubleshooting)
7. [Tips and Best Practices](#tips-and-best-practices)

## Getting Started

### What is SMS/MMS Integration?

Comunicado's mobile integration allows you to send and receive SMS and MMS messages directly from your terminal email client. This feature connects your Android device to Comunicado using KDE Connect, enabling seamless messaging without leaving your terminal environment.

### Key Features

- 📱 **Send and receive SMS messages** from your computer
- 🖼️ **MMS support** for multimedia messages with images and attachments
- 💬 **Conversation management** with threaded message views
- 🔍 **Message search** across all conversations
- 📊 **Read receipts and status tracking**
- 🔕 **Do Not Disturb** scheduling
- 📱 **Multiple device support** for Android phones and tablets

### System Requirements

**Desktop Requirements:**
- Linux operating system (Ubuntu, Fedora, Arch, etc.)
- KDE Connect installed and running
- Comunicado v1.0.0 or later
- Active WiFi network connection

**Mobile Requirements:**
- Android 5.0+ device
- KDE Connect app from Google Play Store or F-Droid
- Same WiFi network as your computer
- SMS permissions granted to KDE Connect

## Setup and Configuration

### Step 1: Install KDE Connect

First, install KDE Connect on your Linux system:

```bash
# Ubuntu/Debian
sudo apt update
sudo apt install kdeconnect

# Fedora
sudo dnf install kdeconnect

# Arch Linux
sudo pacman -S kdeconnect

# openSUSE
sudo zypper install kdeconnect-kde
```

### Step 2: Install Mobile App

1. **Download KDE Connect** from:
   - [Google Play Store](https://play.google.com/store/apps/details?id=org.kde.kdeconnect_tp)
   - [F-Droid](https://f-droid.org/packages/org.kde.kdeconnect_tp/) (open source version)

2. **Grant Permissions**:
   - Open KDE Connect on your phone
   - Allow SMS/MMS permissions when prompted
   - Grant notification access
   - Enable location services (for device discovery)

### Step 3: Pair Your Devices

1. **Start KDE Connect service** on your computer:
   ```bash
   # Start the background service
   systemctl --user start kdeconnect
   
   # Or start manually
   kdeconnect-cli --refresh
   ```

2. **Discover devices**:
   - Open KDE Connect on your phone
   - Your computer should appear in the device list
   - Alternatively, check from command line:
     ```bash
     kdeconnect-cli --list-available
     ```

3. **Pair the devices**:
   - Tap your computer name on your phone
   - Click "Request Pairing"
   - Accept the pairing request on your computer
   - Or use command line:
     ```bash
     kdeconnect-cli --pair --device [DEVICE_ID]
     ```

4. **Verify pairing**:
   ```bash
   # Check paired devices
   kdeconnect-cli --list-available
   
   # Should show your device as "paired and reachable"
   ```

### Step 4: Enable SMS Plugin

1. **Check plugin status**:
   ```bash
   kdeconnect-cli --list-available --device [DEVICE_ID]
   ```

2. **Enable SMS plugin** (if not already enabled):
   - Open KDE Connect on your phone
   - Go to device settings for your computer
   - Enable "SMS" plugin
   - Allow SMS permissions if prompted

3. **Test SMS functionality**:
   ```bash
   kdeconnect-cli --device [DEVICE_ID] --send-sms "Test message" --destination "+1234567890"
   ```

### Step 5: Configure Comunicado

1. **Start Comunicado** with mobile features enabled:
   ```bash
   comunicado
   ```

2. **Access mobile settings**:
   - Press `Ctrl+M` to open mobile interface
   - Or navigate to Settings → Mobile Integration

3. **Configure preferences**:
   - Enable mobile integration
   - Set sync interval (default: 30 seconds)
   - Configure notification preferences
   - Set quiet hours if desired

## Using SMS/MMS Features

### Accessing the Mobile Interface

From the main Comunicado interface:

1. **Keyboard Shortcut**: Press `Ctrl+M`
2. **Menu Navigation**: Navigate to `View` → `Mobile Messages`
3. **Command Mode**: Type `:mobile` and press Enter

### Sending Your First SMS

1. **Open mobile interface** (`Ctrl+M`)
2. **Switch to compose mode** (`C` key)
3. **Enter recipient**: Type phone number (e.g., `+1234567890`)
4. **Type your message** in the message field
5. **Send message**: Press `Ctrl+Enter` or `Enter`

### Reading Messages

1. **View conversations**: Default view shows all conversation threads
2. **Select conversation**: Use arrow keys or `j`/`k` to navigate
3. **Open conversation**: Press `Enter` to view message thread
4. **Mark as read**: Messages are automatically marked as read when viewed
5. **Return to list**: Press `Esc` or `q` to go back

### Message Management

#### Conversation Actions

- **Archive conversation**: Press `A` while conversation is selected
- **Delete conversation**: Press `D` (confirmation required)
- **Search messages**: Press `/` and type search term
- **Refresh messages**: Press `R` to sync with phone

#### Message Actions

- **Reply to message**: Press `R` while in conversation view
- **Forward message**: Press `F` to forward to another contact
- **Copy message text**: Press `C` to copy to clipboard
- **View message details**: Press `I` for timestamps and delivery info

## Interface Guide

### Main Mobile Interface

The mobile interface consists of several view modes:

```
┌─────────────────────────────────────────────────────────────────┐
│ Comunicado - Mobile Messages                            [17:23] │
├─────────────────────────────────────────────────────────────────┤
│ Conversations (3 unread)                                        │
│                                                                 │
│ > John Smith                                     [📱] 2:15 PM   │
│   Hey, are we still meeting today?              (2 unread)     │
│                                                                 │
│   Alice Johnson                                  [📱] 1:45 PM   │
│   Thanks for the documents!                     (read)         │
│                                                                 │
│   Work Group                                     [👥] 12:30 PM  │
│   Meeting scheduled for 3 PM                    (1 unread)     │
│                                                                 │
├─────────────────────────────────────────────────────────────────┤
│ C: Compose  R: Refresh  A: Archive  D: Delete  /: Search       │
│ S: Settings  Q: Quit  ?: Help                                  │
└─────────────────────────────────────────────────────────────────┘
```

### View Modes

1. **Conversation List** (Default)
   - Shows all conversations
   - Unread counts and timestamps
   - Contact names and preview text

2. **Message Thread** (Enter on conversation)
   - Full conversation history
   - Message timestamps
   - Read/delivery status
   - Attachment indicators

3. **Compose Message** (C key)
   - New message creation
   - Recipient selection
   - Message composition
   - Send confirmation

4. **Service Status** (S key)
   - Connection status
   - Sync statistics
   - Device information
   - Error diagnostics

5. **Message Detail** (I key in thread)
   - Full message metadata
   - Delivery timestamps
   - Technical details
   - Attachment information

### Keyboard Shortcuts

#### Global Shortcuts
- `Ctrl+M`: Open mobile interface
- `Esc`: Close current view/go back
- `Q`: Quit mobile interface
- `?`: Show help screen

#### Conversation List
- `↑/↓` or `j/k`: Navigate conversations
- `Enter`: Open selected conversation
- `C`: Compose new message
- `R`: Refresh message list
- `A`: Archive conversation
- `D`: Delete conversation
- `/`: Search conversations
- `S`: Service status
- `1-9`: Quick jump to conversation

#### Message Thread
- `↑/↓` or `j/k`: Navigate messages
- `Enter`: Reply to conversation
- `F`: Forward message
- `C`: Copy message text
- `I`: Message details
- `Page Up/Down`: Scroll messages
- `Home/End`: Jump to start/end

#### Compose Mode
- `Tab`: Switch between fields
- `Ctrl+Enter`: Send message
- `Esc`: Cancel composition
- `Ctrl+V`: Paste from clipboard

### Status Indicators

- `📱`: SMS message
- `🖼️`: MMS with attachments
- `👥`: Group conversation
- `✓`: Message delivered
- `✓✓`: Message read
- `⏳`: Message sending
- `❌`: Message failed
- `🔕`: Do not disturb active

## Advanced Features

### Message Search

Comunicado provides powerful search capabilities:

1. **Quick Search**: Press `/` and type search term
2. **Advanced Search**: Use search operators:
   - `from:John` - Messages from specific contact
   - `date:2025-01-15` - Messages from specific date
   - `unread:true` - Only unread messages
   - `thread:123` - Messages from specific thread

3. **Search Examples**:
   ```
   meeting                    # Find messages containing "meeting"
   from:Alice urgent          # Messages from Alice containing "urgent"
   date:2025-01-15 important  # Important messages from specific date
   ```

### Group Messaging

Managing group conversations:

1. **Create Group**: Compose message with multiple recipients
2. **Add Participants**: Edit group through phone's messaging app
3. **Group Info**: Press `I` in group conversation for details
4. **Leave Group**: Use phone's messaging app (not available in Comunicado)

### Message Filtering

Set up automatic message filtering:

1. **Access Settings**: Press `S` in mobile interface
2. **Configure Filters**:
   - Block spam keywords
   - Priority contact lists
   - Do not disturb schedules
   - Auto-archive rules

3. **Filter Examples**:
   ```toml
   [mobile.notification_filtering]
   keywords_block = ["promotion", "offer", "deal"]
   keywords_allow = ["urgent", "important", "emergency"]
   priority_contacts = ["+1234567890", "+0987654321"]
   ```

### Quiet Hours

Configure do-not-disturb periods:

```toml
[mobile.quiet_hours]
enabled = true
start_time = "22:00"    # 10:00 PM
end_time = "07:00"      # 7:00 AM
timezone = "America/New_York"
weekend_extended = true  # Later start on weekends
emergency_bypass = ["+1234567890"]  # Always allow these numbers
```

### Message Backup and Export

Backup your messages:

1. **Automatic Backup**: Enabled by default
2. **Manual Export**:
   ```bash
   # Export to JSON
   comunicado export-messages --format json --output messages.json
   
   # Export specific conversation
   comunicado export-messages --thread-id 123 --output conversation.json
   ```

3. **Import Messages**:
   ```bash
   # Import from backup
   comunicado import-messages --input messages.json
   ```

### Multiple Device Support

Managing multiple Android devices:

1. **Pair Multiple Devices**: Follow pairing process for each device
2. **Set Primary Device**: Configure preferred device in settings
3. **Switch Devices**: Comunicado automatically uses available device
4. **Device Priorities**: Set preferences in configuration

## Troubleshooting

### Common Issues

#### 1. "KDE Connect not available" Error

**Symptoms**: Cannot access mobile features, error messages about KDE Connect

**Solutions**:
```bash
# Check if KDE Connect is installed
which kdeconnect-cli

# Install if missing
sudo apt install kdeconnect  # Ubuntu/Debian

# Start the service
systemctl --user start kdeconnect
systemctl --user enable kdeconnect  # Auto-start

# Check service status
systemctl --user status kdeconnect
```

#### 2. Device Not Found

**Symptoms**: Phone doesn't appear in device list

**Solutions**:
1. **Check Network Connection**:
   - Ensure both devices on same WiFi network
   - Disable VPN on either device
   - Check firewall settings

2. **Refresh Device List**:
   ```bash
   kdeconnect-cli --refresh
   kdeconnect-cli --list-available
   ```

3. **Restart Services**:
   ```bash
   # On computer
   systemctl --user restart kdeconnect
   
   # On phone: Close and reopen KDE Connect app
   ```

#### 3. Pairing Problems

**Symptoms**: Devices see each other but won't pair

**Solutions**:
1. **Clear Previous Pairings**:
   ```bash
   kdeconnect-cli --unpair --device [DEVICE_ID]
   ```

2. **Check Permissions**: Ensure KDE Connect has all required permissions on phone

3. **Firewall Configuration**:
   ```bash
   # Allow KDE Connect ports
   sudo ufw allow 1714:1764/tcp
   sudo ufw allow 1714:1764/udp
   ```

#### 4. SMS Not Working

**Symptoms**: Devices paired but SMS doesn't work

**Solutions**:
1. **Verify SMS Plugin**:
   ```bash
   kdeconnect-cli --list-available --device [DEVICE_ID]
   # Should show SMS plugin as enabled
   ```

2. **Check Permissions**: 
   - Open KDE Connect on phone
   - Go to device settings
   - Ensure SMS permissions granted

3. **Test Direct SMS**:
   ```bash
   kdeconnect-cli --device [DEVICE_ID] --send-sms "Test" --destination "+1234567890"
   ```

#### 5. Message Sync Issues

**Symptoms**: New messages not appearing, sync delays

**Solutions**:
1. **Check Sync Settings**:
   - Verify sync interval in Comunicado settings
   - Ensure automatic sync is enabled

2. **Manual Refresh**:
   - Press `R` in conversation list
   - Or restart Comunicado

3. **Clear Message Cache**:
   ```bash
   # Stop Comunicado
   # Remove cache files
   rm -rf ~/.config/comunicado/mobile_cache/
   # Restart Comunicado
   ```

### Debug Mode

Enable detailed logging for troubleshooting:

```bash
# Set debug logging
export RUST_LOG=comunicado::mobile=debug,kdeconnect=debug

# Run Comunicado
comunicado

# Or save logs to file
comunicado 2> debug.log
```

### Log Analysis

Key log patterns to look for:

- `KDE Connect availability: true` - Service detected
- `Successfully paired with device` - Pairing successful  
- `Created new conversation` - Message storage working
- `Sync completed: X messages` - Background sync active
- `ERROR` or `WARN` - Issues requiring attention

### Performance Issues

If Comunicado feels slow with mobile features:

1. **Reduce Sync Frequency**:
   ```toml
   [mobile]
   sync_interval_seconds = 60  # Increase from 30 to 60 seconds
   ```

2. **Limit Message History**:
   ```toml
   [mobile]
   storage_retention_days = 30  # Keep only 30 days of messages
   ```

3. **Disable Background Sync**:
   ```toml
   [mobile]
   background_sync_enabled = false  # Manual sync only
   ```

## Tips and Best Practices

### Efficient Messaging Workflow

1. **Use Keyboard Shortcuts**: Learn the key bindings for faster navigation
2. **Search Instead of Scroll**: Use `/` to quickly find conversations
3. **Archive Old Conversations**: Keep active list manageable
4. **Set Quiet Hours**: Avoid interruptions during focused work

### Privacy and Security

1. **Local Storage**: Messages stored locally, never in cloud
2. **Network Security**: Only works on local network
3. **Device Security**: Keep paired devices secure
4. **Permission Review**: Regularly review KDE Connect permissions

### Battery Optimization

Mobile device battery considerations:

1. **Sync Frequency**: Lower sync intervals save battery
2. **WiFi vs Mobile Data**: Use WiFi for better battery life
3. **Background Apps**: Close unnecessary apps on phone
4. **Power Saving**: Enable power saving when needed

### Message Organization

1. **Use Search**: Better than scrolling through conversations
2. **Archive Regularly**: Keep conversation list clean
3. **Group Related**: Use group messages for team communication
4. **Contact Names**: Maintain contact list on phone for better display

### Integration Tips

1. **Email Integration**: Reference SMS conversations in emails
2. **Calendar Integration**: Create events from SMS scheduling
3. **Note Taking**: Copy important messages to notes
4. **Task Management**: Convert messages to tasks

### Performance Optimization

1. **Regular Cleanup**: Periodically clean old messages
2. **Database Maintenance**: Restart Comunicado weekly
3. **Network Quality**: Ensure stable WiFi connection
4. **Resource Monitoring**: Watch CPU/memory usage

## Frequently Asked Questions

### General Questions

**Q: Does this work with iPhones?**
A: No, currently only Android devices are supported through KDE Connect. iPhone support would require different protocols.

**Q: Can I use multiple Android devices?**
A: Yes, you can pair multiple devices. Comunicado will use the first available device for sending messages.

**Q: Does this work over the internet?**
A: No, both devices must be on the same local network. This is a security feature of KDE Connect.

**Q: Are my messages stored in the cloud?**
A: No, all messages are stored locally on your computer. Nothing is sent to external servers.

### Technical Questions

**Q: What happens if KDE Connect stops working?**
A: Comunicado will show a clear error message and disable mobile features until KDE Connect is available again.

**Q: Can I backup my messages?**
A: Yes, messages are stored in SQLite database that can be backed up. Use the export functionality for human-readable backups.

**Q: How much storage do messages use?**
A: Text messages use minimal storage. MMS with images will use more space depending on attachment sizes.

**Q: Can I sync message history from my phone?**
A: KDE Connect only syncs new messages, not historical ones. Your existing phone messages remain on your device.

### Troubleshooting Questions

**Q: Why can't I see my device?**
A: Ensure both devices are on the same WiFi network, KDE Connect is running, and firewall allows connections.

**Q: Messages aren't syncing?**
A: Check that SMS plugin is enabled, permissions are granted, and devices are paired and connected.

**Q: Can I recover deleted messages?**
A: If you have backups enabled, yes. Otherwise, deleted messages cannot be recovered.

## Getting Help

### Documentation Resources

- **User Manual**: This document
- **Technical Guide**: `docs/kde-connect-plugin-guide.md`
- **Configuration Reference**: `docs/configuration.md`
- **API Documentation**: Generated with `cargo doc`

### Community Support

- **GitHub Issues**: Report bugs and request features
- **Discussions**: Ask questions and share tips
- **Wiki**: Community-contributed guides and examples

### Professional Support

For enterprise deployments or custom integrations, professional support is available through the project maintainers.

---

**Last Updated**: August 2, 2025  
**Version**: 1.0.0  
**License**: MIT License

For the latest version of this manual, visit the [Comunicado documentation](https://github.com/user/comunicado/docs/).