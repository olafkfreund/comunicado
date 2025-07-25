# Technical Stack

> Last Updated: 2025-07-25
> Version: 1.0.0

## Core Technologies

### Application Framework
- **Language:** Rust 1.70+
- **Build System:** Nix flakes with devenv
- **Package Manager:** Cargo (Rust native)

### User Interface
- **TUI Framework:** Ratatui (modern Rust TUI library)
- **Terminal Compatibility:** Foot, Kitty, Wezterm, and other modern terminals
- **Input Handling:** Crossterm for cross-platform terminal input

## Email and Calendar Stack

### Email Protocol Implementation
- **IMAP Client:** Custom Rust implementation using tokio for async operations
- **SMTP Support:** Native SMTP for sending emails
- **Email Parsing:** mime crate for RFC-compliant email parsing
- **HTML Rendering:** Custom terminal-compatible HTML parser

### Calendar Protocol Implementation
- **CalDAV Client:** Custom CalDAV implementation for calendar synchronization
- **Calendar Standards:** RFC 5545 (iCalendar) and RFC 4791 (CalDAV) compliance
- **Event Storage:** Local calendar database with CalDAV sync capabilities

### Authentication
- **OAuth2 Implementation:** oauth2 crate for secure authentication
- **Provider Support:** Gmail, Outlook, Yahoo, and custom IMAP servers
- **Token Management:** Secure local token storage with encryption

## Data Storage and Management

### Email Storage
- **Local Storage:** Maildir format for email storage
- **Database:** SQLite for metadata indexing and search
- **Encryption:** Optional email encryption support

### Calendar Storage
- **Local Storage:** iCalendar format for event storage
- **Database:** SQLite for event indexing and queries
- **Sync Strategy:** Bidirectional CalDAV synchronization

## Media and Content Support

### Image and Animation Support
- **Terminal Protocols:** Kitty graphics protocol, Sixel support
- **Image Processing:** image crate for format conversion and resizing
- **Animation Support:** GIF and basic video format support in compatible terminals

### Content Rendering
- **HTML Parser:** Custom lightweight HTML parser optimized for terminal display
- **CSS Support:** Basic CSS styling for terminal-appropriate rendering
- **Font Handling:** Terminal font fallback and Unicode support

## Development and Build Environment

### Development Tools
- **Development Environment:** Nix flakes with devenv for reproducible builds
- **Code Formatting:** rustfmt for consistent code style
- **Linting:** clippy for code quality checks
- **Documentation:** rustdoc for API documentation

### Testing Framework
- **Unit Testing:** Rust's built-in testing framework
- **Integration Testing:** Custom test harness for email/calendar protocol testing
- **Mock Services:** Wiremock for testing against mock IMAP/CalDAV servers

## Configuration and Setup

### Configuration Management
- **Format:** TOML for user configuration files
- **Location:** XDG Base Directory specification compliance
- **Validation:** Serde for configuration parsing and validation

### Setup and Installation
- **Installation:** Cargo install and package manager support (AUR, Nix, etc.)
- **Setup Wizard:** Interactive TUI-based account configuration
- **Migration Tools:** Import from existing email clients and calendar applications

## Platform and Compatibility

### Operating System Support
- **Primary:** Linux (all distributions)
- **Secondary:** macOS and BSD systems
- **Windows:** Limited support via WSL

### Terminal Compatibility
- **Modern Terminals:** Full feature support in Kitty, Foot, Wezterm
- **Legacy Terminals:** Graceful fallback for basic terminals
- **Remote Sessions:** SSH and tmux compatibility

## External Dependencies

### Required System Libraries
- **OpenSSL:** For TLS/SSL connections (or rustls for pure Rust alternative)
- **SQLite:** For local database storage
- **XDG Libraries:** For proper Linux desktop integration

### Optional Dependencies
- **GPG:** For email encryption support
- **Notification Systems:** Desktop notification integration (libnotify)
- **System Calendar:** Integration with Linux desktop calendar systems