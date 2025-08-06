# CLI Plugin Commands Reference

> Comprehensive reference for Notes and KDE Connect plugin CLI commands
> Version: 1.2.0
> Last Updated: 2025-08-06

## Overview

This document covers the command-line interface for Comunicado's built-in plugins:

- **Notes Plugin**: Advanced note-taking and organization system
- **KDE Connect Plugin**: Mobile device integration for SMS/notifications

All plugin commands are accessed through the main `comunicado` CLI with the following syntax:
```bash
comunicado <plugin-name> <command> [options]
```

## Notes Plugin Commands

The Notes plugin provides comprehensive note-taking functionality with CLI access for all operations.

### Basic Commands

#### `notes status`
Show the current configuration and status of the Notes plugin.

```bash
comunicado notes status
```

**Output includes:**
- Configuration directory and settings
- Directory existence and accessibility  
- Watched directories and ignore patterns
- Storage connection status
- Auto-indexing and vim mode settings

#### `notes create <title>`
Create a new note with the specified title.

```bash
comunicado notes create "Meeting Notes" --content "Today's discussion points..."
comunicado notes create "Quick Idea" --tags work,planning --template meeting
```

**Options:**
- `--content <text>` - Note content (required)
- `--tags <tag1,tag2>` - Comma-separated tags
- `--template <name>` - Use a predefined template
- `--dry-run` - Preview the action without creating

**Examples:**
```bash
# Basic note creation
comunicado notes create "Project Ideas" --content "New feature concepts"

# Note with tags
comunicado notes create "Weekly Review" --content "Goals achieved" --tags weekly,review

# From file content
comunicado notes create "Code Review" --content "$(cat review.md)"
```

#### `notes list`
Display all available notes with filtering options.

```bash
comunicado notes list --detailed --tags work --limit 20
```

**Options:**
- `--detailed` - Show extended note information
- `--tags <tag1,tag2>` - Filter by tags
- `--limit <number>` - Maximum number of results

#### `notes show <note>`
Display the content of a specific note.

```bash
comunicado notes show "Meeting Notes"
comunicado notes show "project-ideas" --raw
```

**Options:**
- `--raw` - Show raw markdown without formatting

#### `notes search <query>`
Search through notes using full-text search.

```bash
comunicado notes search "meeting agenda"
comunicado notes search "project" --category work --limit 10
```

**Options:**
- `--limit <number>` - Maximum search results (default: 10)
- `--category <name>` - Search within specific category
- `--titles-only` - Search only in note titles

**Search Examples:**
```bash
# Basic text search
comunicado notes search "development roadmap"

# Tagged search
comunicado notes search "milestone" --category project

# Limited results
comunicado notes search "ideas" --limit 5
```

### Management Commands

#### `notes edit <note>`
Open a note for editing.

```bash
comunicado notes edit "Meeting Notes"
comunicado notes edit "project-plan" --editor vim
```

**Options:**
- `--editor <command>` - Specify editor to use

#### `notes delete <note>`
Delete a note permanently.

```bash
comunicado notes delete "old-notes" --force
```

**Options:**
- `--force` - Delete without confirmation
- `--dry-run` - Show what would be deleted

#### `notes reindex`
Rebuild the search index for better performance.

```bash
comunicado notes reindex --force --verbose
```

**Options:**
- `--force` - Force complete reindex
- `--verbose` - Show detailed progress
- `--dry-run` - Preview reindexing without changes

### Configuration Commands

#### `notes config`
Manage notes plugin configuration.

```bash
comunicado notes config --show
comunicado notes config --set-directory ~/Documents/Notes
comunicado notes config --auto-index true
```

**Options:**
- `--show` - Display current configuration
- `--set-directory <path>` - Change default notes directory
- `--add-watch <path>` - Add directory to watch list
- `--remove-watch <path>` - Remove directory from watch list
- `--auto-index <true|false>` - Enable/disable auto-indexing

### Advanced Commands

#### `notes tui`
Launch the interactive terminal user interface.

```bash
comunicado notes tui
comunicado notes tui --search "meeting"
comunicado notes tui --open "project-notes"
```

**Options:**
- `--search <query>` - Start with search mode
- `--open <note>` - Open specific note directly

#### `notes import`
Import notes from external sources.

```bash
comunicado notes import --format markdown --source ~/old-notes/
comunicado notes import --format obsidian --source vault/ --preserve-structure
```

**Options:**
- `--format <format>` - Source format (markdown, obsidian, etc.)
- `--source <path>` - Source directory or file
- `--preserve-structure` - Maintain directory structure
- `--dry-run` - Preview import without changes

#### `notes export`
Export notes to external formats.

```bash
comunicado notes export --format html --output ~/exported/
comunicado notes export --format markdown --tags work --include-linked
```

**Options:**
- `--format <format>` - Export format (html, markdown, pdf)
- `--output <path>` - Export destination
- `--tags <tag1,tag2>` - Export only tagged notes
- `--include-linked` - Include linked notes

#### `notes stats`
Display statistics about your notes collection.

```bash
comunicado notes stats --detailed --links --index
```

**Options:**
- `--detailed` - Show extended statistics
- `--links` - Include link analysis
- `--index` - Include search index statistics

### Quick Operations

#### `notes quick <content>`
Quickly create a note from command line.

```bash
comunicado notes quick "Remember to buy milk"
comunicado notes quick "Project idea: AI integration" --title "AI Ideas" --tags ai,project
```

**Options:**
- `--title <title>` - Custom note title
- `--tags <tag1,tag2>` - Add tags to the note

#### `notes clipboard`
Create a note from clipboard content.

```bash
comunicado notes clipboard
comunicado notes clipboard --title "Copied Content" --tags clipboard
```

**Options:**
- `--title <title>` - Custom note title
- `--tags <tag1,tag2>` - Add tags to the note

### Integration Commands

#### `notes email-to-note <email-id>`
Convert an email to a note.

```bash
comunicado notes email-to-note msg_12345
```

#### `notes event-to-note <event-id>`
Convert a calendar event to a note.

```bash
comunicado notes event-to-note event_67890
```

## KDE Connect Plugin Commands

The KDE Connect plugin enables integration with mobile devices for SMS, notifications, and device management.

### Status and Information

#### `kde-connect status`
Show current KDE Connect integration status.

```bash
comunicado kde-connect status
```

**Output includes:**
- KDE Connect CLI availability
- Integration enable/disable status
- Connected device information
- Configuration settings
- Pairing and reachability status

#### `kde-connect list`
List all available KDE Connect devices.

```bash
comunicado kde-connect list
```

**Shows:**
- Device names and IDs
- Pairing status (paired/unpaired)
- Reachability (reachable/unreachable)
- Device types and capabilities

### Device Management

#### `kde-connect enable`
Enable KDE Connect integration with automatic device selection.

```bash
comunicado kde-connect enable
comunicado kde-connect enable --device-id abc123 --notifications email,sms
```

**Options:**
- `--device-id <id>` - Specific device to connect to
- `--notifications <type1,type2>` - Notification types to enable
- `--auto-pair` - Automatically pair with unpaired devices
- `--dry-run` - Preview changes without applying

**Example workflows:**
```bash
# Auto-select best available device
comunicado kde-connect enable

# Connect to specific device with custom notifications
comunicado kde-connect enable --device-id myphone123 --notifications sms,email,calendar

# Enable with auto-pairing for new devices
comunicado kde-connect enable --auto-pair
```

#### `kde-connect disable`
Disable KDE Connect integration.

```bash
comunicado kde-connect disable
comunicado kde-connect disable --dry-run
```

**Options:**
- `--dry-run` - Preview changes without applying

#### `kde-connect pair <device-id>`
Pair with a specific device.

```bash
comunicado kde-connect pair abc123def456
```

**Process:**
1. Sends pairing request to device
2. User must accept on mobile device
3. Establishes secure connection
4. Enables communication features

#### `kde-connect unpair <device-id>`
Remove pairing with a device.

```bash
comunicado kde-connect unpair abc123def456
```

### Testing and Setup

#### `kde-connect test`
Test KDE Connect functionality with connected device.

```bash
comunicado kde-connect test
comunicado kde-connect test --notification --find-phone
```

**Options:**
- `--notification` - Send test notification
- `--find-phone` - Trigger find phone feature

**Test scenarios:**
```bash
# Basic connectivity test
comunicado kde-connect test

# Test notification system
comunicado kde-connect test --notification

# Test phone location features
comunicado kde-connect test --find-phone

# Combined functionality test
comunicado kde-connect test --notification --find-phone
```

#### `kde-connect setup`
Run the interactive setup wizard.

```bash
comunicado kde-connect setup
```

**Setup process:**
1. Verifies KDE Connect CLI installation
2. Discovers available devices
3. Guides through pairing process
4. Configures integration settings
5. Enables appropriate plugins

## Error Handling and Troubleshooting

### Common Error Messages

Both plugins now provide enhanced error messages with recovery suggestions:

#### Configuration Errors
```
❌ Configuration Error: Failed to load configuration

💡 Try these solutions:
  • Check your configuration: comunicado config --validate
  • Reset configuration: comunicado config --reset
  • View configuration: comunicado config --show
```

#### Notes Storage Errors
```
❌ Notes Storage Error: Failed to connect to database

💡 Troubleshooting steps:
  • Check directory exists: ~/notes
  • Verify permissions: ls -la ~
  • Create directory: mkdir -p ~/notes
  • Reset notes config: comunicado notes config --reset
```

#### KDE Connect Errors
```
❌ Failed to list KDE Connect devices

💡 Troubleshooting steps:
  • Ensure KDE Connect is running: systemctl --user start kdeconnectd
  • Check network connectivity between devices
  • Try: kdeconnect-cli --refresh
  • Verify device pairing: kdeconnect-cli --list-devices
```

### Plugin Installation Verification

#### Notes Plugin
```bash
# Check if notes plugin is available
comunicado notes status

# Verify configuration
comunicado config --show | grep notes
```

#### KDE Connect Plugin
```bash
# Check KDE Connect availability
comunicado kde-connect status

# Verify system installation
which kdeconnect-cli
systemctl --user status kdeconnectd
```

### Debugging and Logs

#### Enable Debug Mode
```bash
# Set environment variable for detailed logging
export RUST_LOG=comunicado::plugins=debug

# Run commands with debug output
comunicado notes status
comunicado kde-connect list
```

#### Log File Locations
- **Application logs**: `~/.local/share/comunicado/logs/`
- **Plugin logs**: `~/.local/share/comunicado/logs/plugins/`
- **Configuration**: `~/.config/comunicado/config.toml`

## Integration Examples

### Workflow Examples

#### Daily Note Creation
```bash
#!/bin/bash
# Daily note creation script
DATE=$(date +%Y-%m-%d)
comunicado notes create "Daily Log $DATE" \
  --content "# Daily Log - $DATE\n\n## Tasks\n- [ ] \n\n## Notes\n" \
  --tags daily,log
```

#### Mobile Notification Testing
```bash
#!/bin/bash
# Test mobile integration
echo "Testing KDE Connect integration..."
comunicado kde-connect status
if [ $? -eq 0 ]; then
    comunicado kde-connect test --notification
    echo "Test notification sent to mobile device"
fi
```

#### Automated Note Management
```bash
#!/bin/bash
# Weekly note maintenance
echo "Reindexing notes..."
comunicado notes reindex --force

echo "Backing up notes..."
comunicado notes export --format markdown --output ~/backups/notes-$(date +%Y%m%d)

echo "Cleanup completed!"
```

## Configuration Integration

Both plugins integrate with Comunicado's main configuration system:

### Configuration File Structure
```toml
# ~/.config/comunicado/config.toml

[plugins]

[plugins.notes]
enabled = true
default_directory = "~/Documents/Notes"
auto_index = true
vim_mode = true
max_search_results = 100

[plugins.kde_connect]
enabled = true
device_id = "abc123def456"
auto_notifications = true
notification_types = ["email", "sms", "calendar"]
auto_pair = false
```

### Command-Line Configuration
```bash
# Enable both plugins
comunicado config --set plugins.notes.enabled true
comunicado config --set plugins.kde_connect.enabled true

# Configure notes directory
comunicado config --set plugins.notes.default_directory ~/Notes

# Set KDE Connect device
comunicado config --set plugins.kde_connect.device_id myphone123
```

## Future Enhancements

### Planned Features

#### Notes Plugin
- **Full-text search implementation** - Complete search engine integration
- **Template system** - Custom note templates
- **Wiki-style linking** - Bidirectional note linking
- **Export improvements** - Additional export formats
- **Collaborative features** - Note sharing and synchronization

#### KDE Connect Plugin  
- **SMS management** - Send and receive SMS messages
- **File transfers** - Transfer files between devices
- **Remote control** - Control Comunicado from mobile device
- **Battery monitoring** - Display mobile device battery status
- **Clipboard sync** - Synchronize clipboard content

### API Extensions
- **Plugin hooks** - Custom plugin development
- **Event system** - Plugin event notifications
- **External integrations** - Third-party service connections
- **REST API** - HTTP API for external applications

## Support and Resources

### Documentation
- [Plugin Architecture](plugin-architecture.md) - Technical plugin system details
- [Notes Plugin Guide](notes-plugin.md) - Comprehensive notes documentation
- [KDE Connect Integration](kde-connect-plugin-guide.md) - Technical integration guide
- [Configuration Reference](configuration.md) - Complete configuration options

### Community
- **GitHub Issues**: Report bugs and request features
- **Discussions**: Community support and questions
- **Wiki**: Examples and community contributions
- **Discord**: Real-time community support

### Troubleshooting Resources
- [Troubleshooting Guide](troubleshooting.md) - Common issues and solutions
- [Installation Guide](installation.md) - Setup and installation help
- [FAQ](../README.md#faq) - Frequently asked questions

---

*This reference covers all currently implemented CLI functionality for Notes and KDE Connect plugins in Comunicado v1.2.0.*