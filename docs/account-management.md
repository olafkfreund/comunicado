# Account Management

Comunicado supports multiple email accounts and provides comprehensive tools for managing authentication, synchronization, and account-specific settings. This guide covers everything from adding your first account to managing complex multi-account setups.

## Adding Email Accounts

### Automatic Account Setup

For popular email providers, Comunicado can configure accounts automatically with minimal input:

**Supported Providers with Auto-Configuration**
- Gmail and Google Workspace
- Microsoft Outlook.com and Hotmail
- Yahoo Mail
- Apple iCloud Mail
- Most corporate Exchange servers
- Common hosting providers (Fastmail, ProtonMail, etc.)

**Auto-Setup Process**
1. Navigate to Settings (`4`) → Account Management
2. Select "Add New Account"
3. Enter your email address
4. Enter your password or choose OAuth2
5. Wait for automatic server detection
6. Review detected settings and confirm

### Manual Account Configuration

When automatic setup isn't available, configure accounts manually:

**Required Information**
- Your email address
- Email password or app-specific password
- IMAP server settings (for receiving email)
- SMTP server settings (for sending email)

**IMAP Configuration**
- **Server**: IMAP server hostname (e.g., mail.example.com)
- **Port**: Usually 143 (STARTTLS) or 993 (SSL/TLS)
- **Security**: None, STARTTLS, or SSL/TLS
- **Username**: Often your full email address

**SMTP Configuration**
- **Server**: SMTP server hostname
- **Port**: Usually 25, 587 (STARTTLS), or 465 (SSL)
- **Security**: Matching your provider's requirements
- **Authentication**: Usually same as IMAP credentials

### OAuth2 Authentication

Modern authentication method that's more secure than passwords:

**Supported OAuth2 Providers**
- Google (Gmail, Workspace)
- Microsoft (Outlook.com, Office 365)
- Yahoo Mail
- Custom OAuth2 implementations

**OAuth2 Setup Process**
1. Choose OAuth2 during account setup
2. Browser opens automatically for authentication
3. Sign in to your email provider
4. Grant Comunicado necessary permissions
5. Return to terminal - setup completes automatically

**OAuth2 Benefits**
- No password storage in Comunicado
- Revocable access through provider settings
- Automatic token refresh
- Enhanced security features

This comprehensive account management system ensures that Comunicado can handle everything from simple single-account setups to complex enterprise environments with multiple accounts, servers, and integration requirements. The focus on security, flexibility, and ease of use makes it suitable for both personal and professional email management needs.