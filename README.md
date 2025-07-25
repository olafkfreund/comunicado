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

## Project Status

🚧 **Early Development** - This is the foundational TUI interface implementation.

See the [roadmap](.agent-os/product/roadmap.md) for planned features and current progress.

## Contributing

This project follows the Agent OS development workflow. See the documentation in `.agent-os/` for development standards and processes.

## License

MIT or Apache-2.0