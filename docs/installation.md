# Installation Guide

Getting Comunicado up and running on your system is straightforward. This guide covers installation methods for different operating systems and environments.

## System Requirements

Before installing Comunicado, ensure your system meets these requirements:

**Operating System**
- Linux (any modern distribution)
- macOS (10.14 or later)
- Windows (via WSL2)

**Terminal Requirements**
- Any modern terminal emulator
- For best experience: Kitty, Foot, Wezterm, or Alacritty
- UTF-8 support required
- True color support recommended

**Dependencies**
- OpenSSL or compatible TLS library
- SQLite 3.x (usually included)
- System notification daemon (Linux/macOS)

## Installation Methods

### From Pre-built Binaries

The easiest way to install Comunicado is using pre-built binaries:

**Linux (x86_64)**
```bash
curl -L https://github.com/your-username/comunicado/releases/latest/download/comunicado-linux-x86_64.tar.gz | tar xz
sudo mv comunicado /usr/local/bin/
```

**macOS (Intel and Apple Silicon)**
```bash
curl -L https://github.com/your-username/comunicado/releases/latest/download/comunicado-macos.tar.gz | tar xz
sudo mv comunicado /usr/local/bin/
```

### Package Managers

**Arch Linux (AUR)**
```bash
yay -S comunicado
# or
paru -S comunicado
```

**Homebrew (macOS/Linux)**
```bash
brew install comunicado
```

**Nix/NixOS**
```bash
nix-env -iA nixpkgs.comunicado
```

### From Source

If you want the latest features or need to compile for your specific system:

**Prerequisites**
- Rust 1.70 or later
- Git

**Installation Steps**
```bash
# Clone the repository
git clone https://github.com/your-username/comunicado.git
cd comunicado

# Build and install
cargo install --path .
```

**Development Build**
```bash
# For development with all features
git clone https://github.com/your-username/comunicado.git
cd comunicado
cargo build --release

# The binary will be in target/release/comunicado
```

## Post-Installation Setup

After installing Comunicado, you'll need to set it up:

### First Run

Launch Comunicado for the first time:
```bash
comunicado
```

This will start the setup wizard, which will guide you through:
- Creating your first email account
- Setting up basic preferences
- Configuring keyboard shortcuts

### Configuration Directory

Comunicado stores its configuration and data in:
- Linux: `~/.config/comunicado/`
- macOS: `~/Library/Application Support/comunicado/`
- Windows: `%APPDATA%\comunicado\`

### Setting Up Your First Account

The setup wizard will help you configure your first email account. You'll need:

**For Gmail/Google Workspace**
- Your email address
- App password or OAuth2 setup
- IMAP and SMTP server details (auto-detected)

**For Other Providers**
- Email address and password
- IMAP server settings
- SMTP server settings
- Security preferences (TLS/SSL)

Most common providers are auto-detected, but you can also configure custom servers.

## Verification

To verify your installation works correctly:

```bash
# Check version
comunicado --version

# Run basic connectivity test
comunicado --check-config

# Start in test mode (doesn't modify anything)
comunicado --dry-run
```

## Terminal Configuration

For the best experience, consider these terminal settings:

### Recommended Terminal Emulators

**Kitty** (Best overall experience)
```bash
# Install on macOS
brew install kitty

# Install on Linux
curl -L https://sw.kovidgoyal.net/kitty/installer.sh | sh /dev/stdin
```

**Alacritty** (Good performance)
```bash
# Install via package manager
# Ubuntu/Debian
apt install alacritty

# macOS
brew install alacritty
```

### Terminal Settings

Add these to your terminal configuration for optimal display:

**For Kitty** (`~/.config/kitty/kitty.conf`):
```
font_size 12.0
cursor_blink_interval 0
enable_audio_bell no
window_padding_width 4
```

**For Alacritty** (`~/.config/alacritty/alacritty.yml`):
```yaml
font:
  size: 12.0
colors:
  primary:
    background: '0x1e1e1e'
    foreground: '0xd4d4d4'
```

## Troubleshooting Installation

### Common Issues

**Permission denied when installing**
```bash
# If installing system-wide fails, try user installation
cargo install --path . --root ~/.local
# Then add ~/.local/bin to your PATH
```

**Missing dependencies on Linux**
```bash
# Ubuntu/Debian
sudo apt install build-essential libssl-dev pkg-config libsqlite3-dev

# Fedora/RHEL
sudo dnf install gcc openssl-devel sqlite-devel

# Arch Linux
sudo pacman -S base-devel openssl sqlite
```

**macOS compilation issues**
```bash
# Install Xcode command line tools
xcode-select --install

# Or install via Homebrew
brew install openssl sqlite
```

### Getting Help

If you encounter issues during installation:

1. Check the [troubleshooting guide](troubleshooting.md)
2. Search existing [GitHub issues](https://github.com/your-username/comunicado/issues)
3. Create a new issue with:
   - Your operating system and version
   - Terminal emulator and version
   - Complete error messages
   - Steps you've already tried

## Next Steps

Once Comunicado is installed:
1. Follow the [Quick Start Guide](quick-start.md)
2. Set up your [email accounts](account-management.md)
3. Learn the [keyboard shortcuts](keyboard-shortcuts.md)
4. Explore [advanced features](../README.md)

The initial setup might take a few minutes while Comunicado downloads and indexes your emails, but after that, you'll have a fast, efficient terminal-based email client ready to use.