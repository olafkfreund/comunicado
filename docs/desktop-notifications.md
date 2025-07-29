# Desktop Notifications

Comunicado integrates with your system's notification system to keep you informed about important email and calendar events, even when the application isn't in focus or is running in the background.

## Notification Types

### Email Notifications
**New Message Alerts**
Receive immediate notifications when new emails arrive:
- Sender name and email address
- Subject line preview
- Folder location
- Message priority indicators

**Message Status Updates**
Get notified about email delivery and reading confirmations:
- Message sent successfully
- Delivery receipts
- Read receipts (when available)
- Send failure notifications

### Calendar Notifications
**Event Reminders**
Stay on top of your schedule with timely reminders:
- Configurable reminder times (5 minutes to 1 week before)
- Multiple reminders per event
- Recurring event notifications
- All-day event reminders

**RSVP and Invitations**
Never miss meeting invitations:
- New meeting invitations
- RSVP confirmations
- Meeting updates and changes
- Cancellation notifications

**Calendar Sync Status**
Stay informed about calendar synchronization:
- Successful sync completion
- Sync errors or conflicts
- Connection status changes
- Server availability issues

## Platform Support

### Linux Desktop Environments
**GNOME/GTK-based Desktops**
Full integration with the GNOME notification system:
- Native notification appearance
- Action buttons (Reply, Archive, etc.)
- Notification history access
- Do Not Disturb mode respect

**KDE Plasma**
Seamless integration with KNotification:
- Plasma notification style
- System tray integration
- Custom notification sounds
- Notification rules support

**XFCE, LXQt, and Others**
Compatible with standard desktop notification protocols:
- Basic notification display
- Click-to-focus functionality
- System notification settings
- Icon and urgency support

### macOS Integration
**Native macOS Notifications**
Full support for macOS notification center:
- Banner and alert styles
- Notification grouping
- Sound and vibration patterns
- Focus mode integration

**Terminal.app and iTerm2**
Special handling for terminal-based workflows:
- App icon in notifications
- Terminal focus restoration
- Background app notifications
- Notification scheduling

### Windows (WSL2)
**Windows Notification System**
When running under WSL2:
- Windows 10/11 toast notifications
- Action center integration
- Focus assist compatibility
- Cross-system notification sync

## Configuration Options

### Notification Preferences
**Enable/Disable by Type**
Granular control over notification types:
- New email notifications (on/off)
- Calendar reminders (on/off)
- System status notifications (on/off)
- Error and warning notifications (on/off)

**Content Privacy Settings**
Control how much information is shown:
- **Full Preview**: Show sender, subject, and snippet
- **Basic Info**: Show sender and subject only
- **Minimal**: Show only "New message" notification
- **Private**: Show notification count only

### Timing and Frequency
**Notification Batching**
Prevent notification spam during high email volumes:
- Batch multiple messages into single notification
- Configurable batching window (30 seconds to 5 minutes)
- Summary notifications for large batches
- Individual notifications for high-priority messages

**Quiet Hours**
Set times when notifications are suppressed:
- Daily quiet hours (e.g., 10 PM to 8 AM)
- Weekend notification settings
- Holiday and vacation modes
- Meeting mode (during calendar events)

### Visual and Audio Settings
**Notification Appearance**
Customize how notifications look:
- Custom notification icons
- Color coding by account or priority
- Message preview length
- Notification position (if supported)

**Sound Configuration**
Audio feedback for different notification types:
- Custom sound files for different events
- Volume levels per notification type
- Silent mode options
- System sound integration

## Advanced Features

### Smart Notifications
**Priority Detection**
Comunicado analyzes incoming messages to determine importance:
- VIP sender detection
- Keyword-based priority flagging
- Calendar event urgency levels
- Custom priority rules

**Context Awareness**
Notifications adapt to your current activity:
- Reduced frequency during active email sessions
- Meeting mode during calendar events
- Focus mode detection and respect
- Work hours vs personal time differentiation

### Action Integration
**Direct Actions from Notifications**
Handle emails and events directly from notifications:
- Quick reply to emails
- Archive or delete messages
- Snooze calendar reminders
- RSVP to meeting invitations

**Keyboard Shortcuts**
Even notifications support keyboard efficiency:
- System-wide keyboard shortcuts for notification actions
- Quick access to specific notification types
- Batch notification management
- Notification history navigation

## Managing Notification History

### Notification Center
**View Past Notifications**
Access previous notifications through:
- System notification center integration
- In-app notification history
- Search through past notifications
- Mark notifications as read/unread

**Notification Statistics**
Track your notification patterns:
- Daily notification counts
- Most active notification types
- Peak notification times
- Response rate tracking

## Troubleshooting Notifications

### Common Issues
**Notifications Not Appearing**
Check these common causes:
- System notification permissions
- Do Not Disturb or Focus mode status
- Application notification settings
- Background app permissions

**Too Many Notifications**
Reduce notification frequency:
- Enable notification batching
- Set up quiet hours
- Adjust priority thresholds
- Use VIP-only mode

**Missing Action Buttons**
Some notification features require:
- Recent system versions
- Proper permission grants
- Supported desktop environments
- Foreground app permissions

### System-Specific Setup

**Linux Setup**
Ensure proper notification daemon installation:
```bash
# Check if notification daemon is running
ps aux | grep notification

# Test notifications manually
notify-send "Test" "Comunicado notification test"

# Install missing components
sudo apt install libnotify-bin  # Ubuntu/Debian
sudo dnf install libnotify      # Fedora
```

**macOS Permissions**
Grant notification permissions:
1. System Preferences → Notifications & Focus
2. Find Comunicado or Terminal in the list
3. Enable "Allow Notifications"
4. Configure alert style and options

**Windows WSL2**
Enable WSL2 notification forwarding:
- Ensure Windows notification settings allow WSL apps
- Check Windows Focus Assist settings
- Verify WSL2 network connectivity
- Update to latest WSL2 version

## Integration with Email Workflow

### Notification-Driven Workflow
**Immediate Response Mode**
For users who prefer immediate email handling:
- Notifications lead directly to compose window
- Quick action shortcuts from notifications
- Automatic marking of handled notifications
- Seamless return to previous activity

**Batch Processing Mode**
For users who prefer scheduled email processing:
- Notification summaries at set intervals
- Bulk notification review and handling
- Priority-based notification queuing
- Scheduled notification delivery

### Cross-Device Synchronization
**Notification State Sync**
When using Comunicado on multiple devices:
- Read notifications sync across devices
- Prevent duplicate notifications
- Maintain notification history
- Share notification preferences

## Privacy and Security

### Notification Security
**Sensitive Information Protection**
- Automatic detection of sensitive content
- Redacted notifications for financial emails
- Privacy mode for confidential messages
- Secure notification storage

**Network Privacy**
- No cloud-based notification services
- Local notification processing only
- Encrypted notification data storage
- No external notification tracking

### Data Handling
**Notification Data Storage**
- Local-only notification history
- Automatic cleanup of old notifications
- User-controlled data retention
- Secure deletion of notification data

**Permission Management**
- Minimal required system permissions
- User consent for each notification type
- Easy permission revocation
- Regular permission audits

This comprehensive notification system ensures you stay connected to your email and calendar without being overwhelmed, while respecting your privacy and system preferences. The notifications work seamlessly whether you're actively using Comunicado or have it running in the background, making it an effective communication hub for your terminal-based workflow.