# Comunicado

A modern TUI-based email and calendar client for terminal power users.

## Features

- 🖥️ **Modern TUI Interface** - Clean, intuitive design with vim-style navigation
- 📧 **Rich Email Support** - HTML rendering, images, and attachments in terminal
- 🔐 **Secure Authentication** - OAuth2 support for Gmail, Outlook, and more
- 📅 **Integrated Calendar** - CalDAV synchronization and meeting management
- ⚡ **High Performance** - Built with Rust for speed and reliability

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
- `↑`/`↓` - Move up/down in lists
- `Enter` - Select/expand items

### Global
- `q` - Quit application
- `Ctrl+C` - Force quit

### Account Management
- `Ctrl+A` - Add new account
- `Ctrl+X` - Remove account (when account switcher focused)

### Email Composition
- `c` - Compose new email

## Development

This project uses:
- **Language:** Rust 1.70+
- **TUI Framework:** Ratatui
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
- [Account Management](docs/account-management.md) - Adding, switching, and removing email accounts
- [Terminal Compatibility](docs/terminal-compatibility.md) - Image display support across terminal emulators

### Development
- [Product Roadmap](.agent-os/product/roadmap.md) - Current development progress and planned features
- [Technical Architecture](.agent-os/product/tech-stack.md) - Technology choices and architecture decisions

## Project Status

🚀 **Advanced Development** - Feature-complete email and calendar client with comprehensive functionality.

### Current Capabilities
- ✅ **Full Email Management** - IMAP/SMTP with HTML rendering, attachments, and threading
- ✅ **Calendar Integration** - CalDAV sync, Google Calendar API, RSVP handling
- ✅ **Contact Management** - Address book with autocomplete and provider sync
- ✅ **Advanced Features** - Search, filtering, multiple accounts, OAuth2 authentication
- ✅ **Modern TUI** - Vim-style navigation, terminal graphics, optimized performance

See the [roadmap](.agent-os/product/roadmap.md) for detailed progress and upcoming features.

## Contributing

This project follows the Agent OS development workflow. See the documentation in `.agent-os/` for development standards and processes.

## License

MIT or Apache-2.0
