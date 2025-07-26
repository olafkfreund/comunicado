# Account Management Guide

> Complete guide to managing multiple email accounts in Comunicado

## Overview

Comunicado supports unlimited email accounts with full OAuth2 authentication for Gmail, Outlook, Yahoo, and custom IMAP servers. This guide covers adding, switching, and removing accounts.

## Account Operations

### Adding New Accounts

**Shortcut**: `Ctrl+A` (from anywhere in the application)

**Process**:
1. Press `Ctrl+A` to launch the OAuth2 setup wizard
2. Enter email address and optional display name
3. Provider is automatically detected or manually selected
4. Follow provider-specific setup instructions
5. Enter OAuth2 Client ID and optional Client Secret
6. Complete browser authorization
7. Account is tested and added automatically

**Supported Providers**:
- **Gmail** (@gmail.com, @googlemail.com)
- **Outlook** (@outlook.com, @hotmail.com, @live.com)
- **Yahoo** (@yahoo.com, @yahoo.co.uk, and variants)
- **Custom IMAP** (any provider with IMAP/SMTP)

### Switching Between Accounts

**Navigation**:
1. Press `Tab` to focus the account switcher (top-left pane)
2. Use `j/k` or `↑/↓` arrows to select account
3. Press `Enter` to switch to selected account

**Visual Indicators**:
- 🟢 **Online** - Account connected and syncing
- 🟡 **Syncing** - Currently connecting or fetching messages
- ⚫ **Offline** - Account disconnected
- 🔴 **Error** - Connection failed or token expired

**Account Display**:
- Provider icon (📧 Gmail, 📬 Outlook, 📮 Yahoo, 📫 IMAP)
- Display name and email address
- Unread count (if > 0)
- Status indicator

### Removing Accounts

**Shortcut**: `Ctrl+X` (only when account switcher is focused)

**Safety Requirements**:
- Must focus account switcher first (`Tab` to navigate)
- Cannot remove the last remaining account
- Account must be selected before removal

**Process**:
1. Focus account switcher with `Tab`
2. Navigate to account to remove with `j/k`
3. Press `Ctrl+X` to remove selected account
4. Account is immediately removed with full cleanup

**Data Removal**:
- OAuth2 tokens from system keyring
- Account configuration files
- All emails and folders from local database
- Account from user interface

## OAuth2 Setup Details

### Gmail Setup

**Requirements**:
- Google Cloud Console project
- OAuth2 Client ID (and optionally Client Secret)
- Enabled Gmail API

**Steps**:
1. Go to [Google Cloud Console](https://console.cloud.google.com/)
2. Create project or select existing
3. Enable Gmail API
4. Create OAuth2 credentials
5. Add `http://localhost:8080/oauth/callback` as redirect URI
6. Copy Client ID and Client Secret

### Outlook Setup

**Requirements**:
- Azure App Registration
- Application (client) ID
- Optionally client secret for enhanced security

**Steps**:
1. Go to [Azure Portal](https://portal.azure.com/)
2. Navigate to App registrations
3. Create new registration
4. Add `http://localhost:8080/oauth/callback` as redirect URI
5. Configure Mail.Read and Mail.Send permissions
6. Copy Application ID and client secret

### Yahoo Setup

**Requirements**:
- Yahoo Developer account
- App registration with Yahoo

**Steps**:
1. Go to [Yahoo Developer Network](https://developer.yahoo.com/)
2. Create app with Mail permissions
3. Add callback URL: `http://localhost:8080/oauth/callback`
4. Copy Client ID and Client Secret

### Custom IMAP

**Requirements**:
- IMAP/SMTP server details
- OAuth2 support (if available) or app passwords

**Configuration**:
- Server hostnames and ports
- Security method (TLS/SSL)
- Authentication method

## Account Switcher Features

### Expanded View

**Toggle**: Press `Space` when account switcher is focused

**Shows**:
- All accounts with full details
- Provider icons and status
- Unread counts
- Account selection highlighting

### Collapsed View (Default)

**Shows**:
- Current account only
- Expansion indicator (▼) if multiple accounts
- Quick status overview

### Navigation

| Key | Action |
|-----|--------|
| `Tab` | Focus account switcher |
| `j/k` or `↑/↓` | Navigate accounts |
| `Space` | Toggle expanded/collapsed view |
| `Enter` | Switch to selected account |
| `Ctrl+A` | Add new account |
| `Ctrl+X` | Remove selected account |

## Troubleshooting

### Token Expired (🔴 Red Status)

**Symptoms**:
- Account shows red status indicator
- Cannot fetch new messages
- IMAP connection fails

**Solutions**:
1. **Auto-refresh**: Try switching to the account (may auto-refresh)
2. **Manual refresh**: Remove and re-add the account
3. **Check credentials**: Verify OAuth2 setup is still valid

### Connection Issues

**Common Causes**:
- Internet connectivity problems
- OAuth2 token revoked by provider
- Provider API changes or outages
- Firewall blocking IMAP/SMTP ports

**Diagnosis**:
- Check other accounts (isolate account-specific issues)
- Verify internet connection
- Check provider status pages
- Review application logs

### Account Not Switching

**Symptoms**:
- Selection doesn't change active account
- Messages don't update
- UI shows wrong account

**Solutions**:
- Ensure account switcher is focused before pressing Enter
- Check account status (may be offline)
- Restart application if persistent

## Best Practices

### Security

- **Use OAuth2** whenever possible (more secure than passwords)
- **Review permissions** granted to Comunicado in provider settings
- **Revoke access** for unused accounts in provider dashboards
- **Keep credentials secure** - never share Client IDs/Secrets

### Organization

- **Meaningful names** - Use descriptive display names for accounts
- **Account grouping** - Organize by purpose (work, personal, etc.)
- **Regular cleanup** - Remove unused accounts to reduce clutter

### Performance

- **Limit accounts** - Too many accounts may impact performance
- **Monitor sync** - Watch for accounts with frequent connection issues
- **Cache management** - Application handles caching automatically

## Advanced Configuration

### Multiple Accounts Same Provider

You can add multiple Gmail/Outlook accounts:
- Each needs separate OAuth2 setup
- Use different display names for clarity
- All share same provider configuration

### Custom IMAP Servers

For non-OAuth2 providers:
- Use app-specific passwords when available
- Configure IMAP/SMTP settings manually
- May require additional security setup

### Environment Variables

Configure default behavior:
```bash
export COMUNICADO_DEFAULT_ACCOUNT="work@example.com"
export COMUNICADO_OAUTH_REDIRECT_PORT="8080"
```

---

*For OAuth2 setup guides and provider-specific instructions, refer to the [OAuth2 Setup Documentation](oauth2-setup.md).*