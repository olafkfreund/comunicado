# Quick Start Guide

This guide gets you up and running with Comunicado in just a few minutes. By the end, you'll have configured your first email account and be ready to start managing your email and calendar from the terminal.

## Before You Begin

Make sure you have:
- Comunicado installed on your system (see [Installation Guide](installation.md))
- Your email account credentials ready
- A modern terminal emulator (recommended: Kitty, Alacritty, or Wezterm)
- About 10 minutes for the initial setup

## First Launch

Open your terminal and start Comunicado:

```bash
comunicado
```

If this is your first time running Comunicado, you'll see the welcome screen and setup wizard. Don't worry - the wizard will guide you through everything you need to get started.

## Account Setup

### Automatic Configuration

For most popular email providers, Comunicado can configure everything automatically. When prompted:

1. **Enter your email address** - Type your full email address
2. **Enter your password** - Use your regular email password or app-specific password
3. **Wait for auto-detection** - Comunicado will detect server settings

**Supported Auto-Configuration**
- Gmail and Google Workspace
- Outlook.com and Hotmail
- Yahoo Mail
- iCloud Mail
- Most major corporate email systems

### Manual Configuration

If auto-configuration doesn't work, you'll need to enter server details manually:

**IMAP Settings** (for receiving email)
- Server: Your IMAP server address (e.g., imap.gmail.com)
- Port: Usually 993 for SSL/TLS
- Security: Select SSL/TLS for security

**SMTP Settings** (for sending email)
- Server: Your SMTP server address (e.g., smtp.gmail.com)
- Port: Usually 587 for STARTTLS or 465 for SSL
- Security: Select appropriate security method

**Finding Your Settings**
Most email providers publish their settings online:
- Search for "[your provider] IMAP SMTP settings"
- Check your provider's support documentation
- Look in your existing email client's account settings

### OAuth2 Setup

For Gmail and some other providers, you can use OAuth2 for enhanced security:

1. **Choose OAuth2** when prompted for authentication method
2. **Browser opens** automatically for authentication
3. **Sign in** to your email account in the browser
4. **Grant permissions** to Comunicado
5. **Return to terminal** - setup completes automatically

OAuth2 is more secure than using your password and is the recommended method for supported providers.

## Initial Synchronization

After account setup, Comunicado begins synchronizing your email:

**What Happens During Sync**
- Downloads your folder structure
- Fetches recent messages (last 30 days by default)
- Indexes messages for search
- Sets up local database

**First Sync Duration**
- Small accounts (< 1000 messages): 1-2 minutes
- Medium accounts (1000-10000 messages): 5-10 minutes
- Large accounts (> 10000 messages): 15+ minutes

You can start using Comunicado while synchronization continues in the background.

## Basic Navigation

Once Comunicado loads, you'll see the main interface:

**Left Panel**: Folder list
- Shows all your email folders
- Numbers in parentheses indicate unread messages
- Use `j`/`k` or arrow keys to navigate

**Center Panel**: Message list
- Displays messages in the selected folder
- Use `j`/`k` or arrow keys to move between messages
- Press `Enter` to read a message

**Right Panel**: Message preview
- Shows the content of the selected message
- Updates automatically as you navigate

**Status Bar**: Bottom of screen
- Shows current status and available shortcuts
- Displays sync progress and connection status

## Essential Shortcuts

Learn these basic shortcuts to get started:

**Navigation**
- `j` / `k` - Move up/down in lists
- `Enter` - Open selected item
- `Esc` - Go back or cancel
- `Tab` - Switch between panels

**Email Actions**
- `c` - Compose new message
- `r` - Reply to message
- `f` - Forward message
- `d` - Delete message
- `a` - Archive message

**Application**
- `?` - Show help
- `Ctrl+Q` - Quit Comunicado
- `Ctrl+R` - Refresh current view

## Reading Your First Email

Let's read an email to get familiar with the interface:

1. **Select a folder** - Use `j`/`k` to highlight your Inbox
2. **Press Enter** - This opens the Inbox folder
3. **Select a message** - Use `j`/`k` to highlight a message
4. **Read in preview** - The message content appears in the right panel
5. **Full view** - Press `Enter` to open the message in full view
6. **Go back** - Press `Esc` to return to the message list

**Message View Features**
- Scroll with `Space` (down) and `Shift+Space` (up)
- Toggle header display with `h`
- View raw message source with `v`
- See attachments with `A`

## Sending Your First Email

Let's compose and send a test message:

1. **Start composing** - Press `c` from anywhere
2. **Fill in recipient** - Type an email address in the "To" field
3. **Move to subject** - Press `Tab` to move to the subject field
4. **Enter subject** - Type a subject line
5. **Move to body** - Press `Tab` to move to the message body
6. **Write message** - Type your message content
7. **Send** - Press `Ctrl+Enter` to send the message

**Compose Window Tips**
- Use `Tab` to move between fields
- `Ctrl+S` saves as draft
- `Ctrl+X` discards the message
- `Esc` returns to the previous screen

## Setting Up Calendar (Optional)

If you want to use calendar features:

1. **Open settings** - Press `4` or navigate to Settings
2. **Calendar setup** - Select calendar configuration
3. **Add calendar** - Choose your calendar provider
4. **Authentication** - Sign in to your calendar service
5. **Sync** - Wait for initial calendar sync

Calendar integration works with Google Calendar, iCloud, Exchange, and any CalDAV server.

## Customizing Your Experience

### Basic Preferences

Access preferences to customize Comunicado:

1. **Open settings** - Press `4`
2. **Preferences** - Select general preferences
3. **Adjust settings** like:
   - Email check frequency
   - Notification preferences
   - Display options
   - Keyboard shortcuts

### Theme Selection

Choose a color theme that works for your terminal:

1. **Theme settings** - In settings, select themes
2. **Browse themes** - Use arrow keys to see options
3. **Preview** - Themes update in real-time
4. **Apply** - Press `Enter` to confirm

Popular themes include:
- **Default** - Works well in most terminals
- **Dark** - For dark terminal backgrounds
- **High Contrast** - For better accessibility
- **Solarized** - Popular programmer theme

## Common First-Day Tasks

Here are typical things new users want to do:

### Organizing Email
- Create new folders: `Ctrl+N` in folder list
- Move messages: Select message, press `m`, choose destination
- Search messages: Press `/` and type search terms

### Managing Multiple Accounts
- Add second account: In settings, add another account
- Switch accounts: Use `Ctrl+1`, `Ctrl+2`, etc.
- Unified inbox: View messages from all accounts together

### Setting Up Notifications
- Enable desktop notifications in preferences
- Configure notification rules and quiet hours
- Test notifications with a test message

## Getting Help

If you need assistance:

**Built-in Help**
- Press `?` or `F1` for context-sensitive help
- Status bar shows available shortcuts
- Help screens explain features in detail

**Documentation**
- This documentation covers all features comprehensively
- Check specific feature guides for detailed information
- Look at troubleshooting guides for common issues

**Community Support**
- GitHub issues for bug reports and feature requests
- Community forums for usage questions
- Email support for account-specific problems

## Next Steps

Now that you have Comunicado running:

1. **Learn more shortcuts** - Check the [Keyboard Shortcuts](keyboard-shortcuts.md) guide
2. **Explore email features** - Read about [Email Management](email-management.md)
3. **Set up advanced features** - Try [Import/Export](import-export.md) or [Desktop Notifications](desktop-notifications.md)
4. **Customize your workflow** - Adjust settings and shortcuts to match your preferences

## Troubleshooting Quick Issues

**Can't connect to email server**
- Check your internet connection
- Verify server settings are correct
- Try disabling antivirus email scanning temporarily

**Messages not downloading**
- Check if folders are selected for sync
- Look at sync status in the status bar
- Try manually refreshing with `Ctrl+R`

**Interface looks wrong**
- Try a different color theme
- Check your terminal's color settings
- Ensure your terminal supports UTF-8

**Keyboard shortcuts not working**
- Make sure Comunicado window has focus
- Check if your terminal is intercepting shortcuts
- Review shortcut customizations in settings

Most issues resolve quickly, and Comunicado is designed to work reliably out of the box. The terminal-based interface might feel different at first, but most users find it much faster than traditional email clients once they learn the basics.

Welcome to Comunicado! You now have a powerful, efficient email and calendar system running entirely in your terminal. The keyboard-driven interface will make your email management much faster once you get comfortable with the shortcuts.