# Comunicado

A modern TUI-based email and calendar client for terminal power users and privacy-conscious developers.

## Features

- **Modern TUI Interface** - Clean, intuitive design with vim-style navigation
- **Rich Email Support** - HTML rendering, images, and attachments in terminal
- **GPG Encryption** - Complete PGP encryption with Sequoia-PGP backend
- **Secure Authentication** - OAuth2 support for Gmail, Outlook, and more
- **Integrated Calendar** - CalDAV synchronization and meeting management
- **High Performance** - Built with Rust for speed and reliability

## Quick Start

### Using Nix Flakes

```bash
# Development environment
nix develop

# Build and run
just run

# Install
nix profile install .
```

### Using Cargo

```bash
# Build
cargo build --release

# Run
cargo run
```

## Keyboard Shortcuts

### Navigation
- `Tab` / `Shift+Tab` - Switch between panes
- `h`/`j`/`k`/`l` - Vim-style movement
- Arrow keys - Move up/down in lists
- `Enter` - Select/expand items

### Global
- `q` - Quit application
- `Ctrl+C` - Force quit
- `Ctrl+D` - Command palette

### Account Management
- `Ctrl+A` - Add new account
- `Ctrl+X` - Remove account (when account switcher focused)

### Email
- `c` - Compose new email
- `r` - Reply to email
- `f` - Forward email
- `D` - Decrypt encrypted email (in viewer)

### Encryption
- `Ctrl+E` - Toggle encryption controls (in compose)

## Security & Encryption

Comunicado provides enterprise-grade GPG encryption capabilities:

- **Real Cryptographic Operations** - Powered by Sequoia-PGP
- **Key Management** - Import, export, and generate PGP keys
- **Visual Security Indicators** - See encryption status at a glance
- **Interactive Decryption** - Decrypt emails directly in the viewer
- **Compose Integration** - Encrypt and sign emails during composition

### GPG Setup

1. **Generate Keys**: Use the built-in key generation or import existing keys
2. **Configure Recipients**: Keys are automatically discovered for email addresses
3. **Compose Encrypted**: Use Ctrl+E in compose mode to configure encryption
4. **View Encrypted**: Press D in email viewer to decrypt messages

## Development

This project uses:
- **Language:** Rust 1.70+
- **TUI Framework:** Ratatui
- **Encryption:** Sequoia-PGP
- **Build System:** Nix flakes + devenv
- **Testing:** Cargo test + integration tests

### Development Commands

```bash
just build         # Build the project
just test          # Run tests
just lint          # Run clippy
just fmt           # Format code
just check         # Run all checks
```

## Documentation

### User Guides
- [Installation Guide](docs/installation.md) - Setup and installation instructions
- [Quick Start Guide](docs/quick-start.md) - Get started with Comunicado
- [Account Management](docs/account-management.md) - Adding, switching, and removing email accounts
- [Calendar Features](docs/calendar-features.md) - Using the integrated calendar
- [Terminal Compatibility](docs/terminal-compatibility.md) - Image display support across terminal emulators

### Method Documentation
- [Method Documentation Overview](docs/method-documentation.md) - Complete codebase analysis
- [Core Methods](docs/core-methods.md) - Main application methods
- [Email Methods](docs/email-methods.md) - Email system functionality
- [Calendar Methods](docs/calendar-methods.md) - Calendar integration
- [Encryption Methods](docs/encryption-methods.md) - GPG encryption system
- [UI Methods](docs/ui-methods.md) - User interface components

### Development
- [Product Roadmap](.agent-os/product/roadmap.md) - Current development progress and planned features
- [Technical Architecture](.agent-os/product/tech-stack.md) - Technology choices and architecture decisions
- [Configuration Guide](docs/configuration.md) - Configuration options and settings

## Project Status

**Advanced Development** - Feature-complete email and calendar client with comprehensive functionality.

### Phase 6 Complete: Advanced Enterprise GPG Features

**Latest Achievement:** Complete GPG encryption system with real cryptographic operations

### Current Capabilities

**Core Email Features**
- Full Email Management - IMAP/SMTP with HTML rendering, attachments, and threading
- Advanced Search - Multi-criteria search with filtering and indexing
- Email Threading - Complete JWZ and Simple threading algorithms
- Multiple Accounts - Support for Gmail, Outlook, and custom IMAP servers

**Calendar Integration**
- CalDAV Synchronization - Complete bidirectional sync with conflict resolution
- Google Calendar API - Full CRUD operations with event management
- Meeting Invitations - RSVP handling with SMTP integration
- Multiple Views - Day, week, month, and agenda view modes

**Security & Encryption**
- GPG Encryption - Complete PGP implementation with Sequoia-PGP
- Key Management - Generate, import, export PGP keys
- Visual Security - Encryption status indicators throughout UI
- Interactive Decryption - Decrypt emails directly in viewer

**Advanced Features**
- Contact Management - Address book with autocomplete and provider sync
- OAuth2 Authentication - Secure login for major email providers
- Terminal Graphics - Image and animation support in compatible terminals
- Plugin Architecture - Extensible system for community plugins
- Desktop Notifications - System notification integration

**Modern TUI Experience**
- Vim-style Navigation - Familiar keyboard shortcuts and movements
- Command Palette - Quick access to all features with Ctrl+D
- Customizable Interface - Theme and layout customization
- Performance Optimized - Efficient rendering and background processing

### Architecture Highlights

- **Total Methods:** 3,031 across the entire codebase
- **Module Structure:** 15+ specialized modules for email, calendar, encryption, UI
- **Backend Abstraction:** Trait-based design supporting multiple implementations
- **Type Safety:** Comprehensive Rust type system with error handling
- **Async Architecture:** Non-blocking operations with proper concurrency

See the [roadmap](.agent-os/product/roadmap.md) for detailed progress and upcoming features.

## Contributing

This project follows the Agent OS development workflow. See the documentation in `.agent-os/` for development standards and processes.

### Code Quality

- **Comprehensive Documentation:** 3,031+ methods documented across 9 specialized guides
- **Type Safety:** Full Rust type system with comprehensive error handling
- **Performance:** Optimized for terminal environments with efficient rendering
- **Testing:** Integration tests and comprehensive method coverage analysis

## License

AGPL-3.0