# Configuration Guide

Comunicado offers extensive configuration options to customize your email and calendar experience. This guide covers all configuration aspects, from basic preferences to advanced customization.

## Configuration File Locations

Comunicado stores configuration files in standard system directories:

**Linux**
```
~/.config/comunicado/
├── config.toml          # Main configuration
├── accounts.toml        # Account settings
├── shortcuts.toml       # Keyboard shortcuts
├── themes/              # Custom themes
└── databases/           # Email and calendar data
```

**macOS**
```
~/Library/Application Support/comunicado/
├── config.toml
├── accounts.toml
├── shortcuts.toml
├── themes/
└── databases/
```

**Windows (WSL)**
```
%APPDATA%\comunicado\
├── config.toml
├── accounts.toml
├── shortcuts.toml
├── themes\
└── databases\
```

## Main Configuration File

The primary configuration file (`config.toml`) contains global settings:

### Basic Settings

```toml
# General application settings
[general]
# Default view when starting Comunicado
default_view = "email"  # Options: email, calendar, contacts

# Check for new messages every N seconds
sync_interval = 300

# Maximum number of recent messages to keep in memory
message_cache_size = 1000

# Enable desktop notifications
notifications_enabled = true

# Show message previews in notifications
notification_previews = true

# Terminal graphics support (auto-detect, force, disable)
graphics_mode = "auto"
```

### Display Settings

```toml
[display]
# Color theme name
theme = "default"

# Show line numbers in message lists
show_line_numbers = true

# Date format for message lists
date_format = "%Y-%m-%d %H:%M"

# Maximum width for message preview
preview_width = 80

# Show folder tree expanded by default
folders_expanded = true

# Message list columns to display
message_columns = ["status", "from", "subject", "date", "size"]
```

### Performance Settings

```toml
[performance]
# Number of messages to load per page
messages_per_page = 50

# Maximum concurrent IMAP connections per account
max_connections = 3

# Search index update frequency (seconds)
search_index_interval = 600

# Enable message threading
threading_enabled = true

# Background sync during active use
background_sync = true
```

### Network Settings

```toml
[network]
# Connection timeout in seconds
timeout = 30

# Enable connection keep-alive
keepalive = true

# Retry failed connections (attempts)
retry_count = 3

# Use system proxy settings
use_system_proxy = true

# Custom proxy (optional)
# proxy = "http://proxy.example.com:8080"
```

## Account Configuration

Email accounts are configured in `accounts.toml`:

### IMAP Account Example

```toml
[[account]]
name = "Work Email"
email = "user@company.com"
display_name = "John Doe"

[account.imap]
server = "imap.company.com"
port = 993
security = "SSL"
username = "user@company.com"
# Password stored in system keychain

[account.smtp]
server = "smtp.company.com"
port = 587
security = "STARTTLS"
username = "user@company.com"

[account.settings]
# Folders to sync (empty = all folders)
sync_folders = ["INBOX", "Sent", "Drafts"]

# Download message bodies automatically
auto_download = true

# Maximum message age to sync (days, 0 = all)
max_age_days = 365

# Default signature
signature = """
Best regards,
John Doe
Senior Developer
Company Name
"""
```

### OAuth2 Account Example

```toml
[[account]]
name = "Gmail"
email = "user@gmail.com"
display_name = "Personal"

[account.oauth2]
provider = "gmail"
client_id = "your-client-id"
# Client secret and tokens stored securely

[account.imap]
server = "imap.gmail.com"
port = 993
security = "SSL"

[account.smtp]
server = "smtp.gmail.com"
port = 587
security = "STARTTLS"
```

### Exchange/Office365 Account

```toml
[[account]]
name = "Office 365"
email = "user@company.onmicrosoft.com"
display_name = "Work Account"

[account.oauth2]
provider = "microsoft"
tenant_id = "your-tenant-id"

[account.imap]
server = "outlook.office365.com"
port = 993
security = "SSL"

[account.smtp]
server = "smtp.office365.com"
port = 587
security = "STARTTLS"

[account.calendar]
# CalDAV URL for calendar sync
caldav_url = "https://outlook.office365.com/owa/calendar/"
```

## Keyboard Shortcuts Configuration

Customize shortcuts in `shortcuts.toml`:

```toml
# Global application shortcuts
[global]
quit = "Ctrl+Q"
help = "F1"
refresh = "Ctrl+R"
search = "/"
command_palette = ":"

# Email-specific shortcuts
[email]
compose = "c"
reply = "r"
reply_all = "R"
forward = "f"
delete = "d"
archive = "a"
mark_read = "u"
flag = "*"

# Navigation shortcuts
[navigation]
next_message = "j"
prev_message = "k"
first_message = "g g"
last_message = "G"
next_folder = "]"
prev_folder = "["
open_folder = "Enter"

# Calendar shortcuts
[calendar]
new_event = "n"
edit_event = "e"
delete_event = "d"
today = "t"
next_week = "l"
prev_week = "h"
month_view = "m"
week_view = "w"
day_view = "d"
```

### Custom Shortcut Syntax

Shortcuts support various key combinations:

```toml
# Single keys
shortcut = "a"

# Ctrl combinations
shortcut = "Ctrl+a"

# Alt combinations  
shortcut = "Alt+a"

# Shift combinations
shortcut = "Shift+a"

# Multiple modifiers
shortcut = "Ctrl+Shift+a"

# Function keys
shortcut = "F1"

# Special keys
shortcut = "Enter"
shortcut = "Escape" 
shortcut = "Space"
shortcut = "Tab"

# Key sequences
shortcut = "g g"  # Press 'g' twice
shortcut = "Ctrl+x Ctrl+s"  # Emacs-style sequences
```

## Theme Configuration

Create custom themes in the `themes/` directory:

### Custom Theme Example (`themes/my-theme.toml`)

```toml
[meta]
name = "My Custom Theme"
description = "A personalized color scheme"
author = "Your Name"

[colors]
# Basic colors
background = "#1e1e1e"
foreground = "#d4d4d4"
cursor = "#ffffff"

# UI elements
border = "#3c3c3c"
selection = "#264f78"
search_highlight = "#613315"

# Message list colors
unread_message = "#ffffff"
read_message = "#cccccc"
important_message = "#ff6b6b"
sender_name = "#4fc3f7"
subject = "#a8e6cf"
date = "#ffd93d"

# Content colors
link = "#4fc3f7"
quote = "#6c757d"
header = "#28a745"
attachment = "#fd7e14"

# Status colors
success = "#28a745"
warning = "#ffc107"
error = "#dc3545"
info = "#17a2b8"
```

### Theme Inheritance

Themes can inherit from other themes:

```toml
[meta]
name = "Dark Blue"
base = "default"  # Inherit from default theme

[colors]
# Only override specific colors
background = "#001122"
selection = "#003366"
```

## Advanced Settings

### Message Processing

```toml
[message_processing]
# Enable spam filtering
spam_filtering = true

# Spam threshold (0.0 to 1.0)
spam_threshold = 0.8

# Auto-delete spam after N days
spam_retention_days = 30

# Enable message threading
threading = true

# Thread grouping sensitivity
thread_sensitivity = "normal"  # strict, normal, loose

# HTML to text conversion quality
html_conversion = "high"  # low, normal, high

# Content sanitization level
sanitization_level = "standard"  # minimal, standard, strict
```

### Security Settings

```toml
[security]
# Encrypt local database
database_encryption = true

# Require password for sensitive operations
require_password = false

# Auto-lock after inactivity (minutes, 0 = disabled)
auto_lock_timeout = 0

# Remember OAuth tokens (less secure but convenient)
remember_oauth_tokens = true

# Check TLS certificates strictly
strict_tls = true

# Allow self-signed certificates (not recommended)
allow_self_signed = false
```

### Calendar Configuration

```toml
[calendar]
# Default calendar view
default_view = "month"  # day, week, month, agenda

# First day of week (0 = Sunday, 1 = Monday)
first_day_of_week = 1

# Working hours start/end (24-hour format)
work_start = "09:00"
work_end = "17:00"

# Working days (0 = Sunday, 6 = Saturday)
work_days = [1, 2, 3, 4, 5]

# Default event duration (minutes)
default_event_duration = 60

# Default reminder time (minutes before event)
default_reminder = 15

# Time zone handling
timezone = "auto"  # auto, local, or specific timezone

# Calendar sync interval (seconds)
sync_interval = 300
```

### Import/Export Settings

```toml
[import_export]
# Default export format
default_export_format = "maildir"

# Include attachments in exports
export_attachments = true

# Maildir folder structure
maildir_structure = "standard"  # standard, hierarchical

# Export date range limit (days, 0 = unlimited)
export_date_limit = 0

# Compression for large exports
compress_exports = true

# Import duplicate handling
duplicate_handling = "skip"  # skip, replace, keep_both
```

## Environment Variables

Override configuration with environment variables:

```bash
# Override config file location
export COMUNICADO_CONFIG_DIR="/custom/config/path"

# Set debug logging level
export COMUNICADO_LOG_LEVEL="debug"

# Force specific theme
export COMUNICADO_THEME="dark"

# Disable graphics
export COMUNICADO_GRAPHICS="false"

# Override sync interval
export COMUNICADO_SYNC_INTERVAL="60"
```

## Configuration Management

### Backup and Restore

**Backup Configuration**
```bash
# Backup entire config directory
tar -czf comunicado-config-backup.tar.gz ~/.config/comunicado/

# Backup only configuration files (no data)
tar -czf comunicado-settings.tar.gz ~/.config/comunicado/*.toml ~/.config/comunicado/themes/
```

**Restore Configuration**
```bash
# Restore complete backup
tar -xzf comunicado-config-backup.tar.gz -C ~/

# Restore only settings
tar -xzf comunicado-settings.tar.gz -C ~/
```

### Configuration Validation

Comunicado includes configuration validation:

```bash
# Check configuration syntax
comunicado --check-config

# Validate specific configuration file
comunicado --validate-config ~/.config/comunicado/config.toml

# Show current configuration values
comunicado --show-config
```

### Configuration Migration

When updating Comunicado, configuration files may need migration:

```bash
# Migrate configuration to new version
comunicado --migrate-config

# Show configuration changes needed
comunicado --config-diff
```

### Sharing Configuration

**Version Control**
```bash
# Track configuration in git
cd ~/.config/comunicado
git init
git add *.toml themes/
git commit -m "Initial Comunicado configuration"
```

**Syncing Across Machines**
- Use dotfiles repositories
- Sync via cloud storage (encrypt sensitive data)
- Export/import configuration through Comunicado settings

### Configuration Best Practices

**Security**
- Never store passwords in configuration files
- Use OAuth2 when available
- Enable database encryption for sensitive environments
- Regular security audits of configuration

**Organization**
- Comment configuration changes with reasons
- Use descriptive account names
- Group related settings together
- Document custom shortcuts and themes

**Performance**
- Adjust cache sizes based on system resources
- Configure sync intervals based on usage patterns
- Enable only needed features to reduce overhead
- Monitor resource usage and adjust accordingly

This comprehensive configuration system allows you to tailor Comunicado exactly to your needs, whether you're looking for simple customization or complex enterprise deployment scenarios.