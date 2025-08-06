# Installation Guide

> Comunicado v0.1.0 Installation Instructions
> Last Updated: August 2025

## Quick Start

Comunicado is available for major Linux distributions. Choose your preferred installation method:

| Distribution | Method | Command |
|--------------|--------|---------|
| **NixOS** | Flakes | `nix run github:olafkfreund/comunicado` |
| **Arch Linux** | AUR | `paru -S comunicado` or `yay -S comunicado` |
| **Debian/Ubuntu** | .deb package | Download from releases |
| **Fedora/RHEL** | .rpm package | Download from releases |
| **Any Linux** | Cargo | `cargo install --git https://github.com/olafkfreund/comunicado` |

## Distribution-Specific Installation

### 🐧 NixOS / Nix Package Manager

**Recommended for NixOS users and those with Nix installed.**

#### Option 1: Direct Run (Try Before Installing)
```bash
# Run Comunicado directly from GitHub
nix run github:olafkfreund/comunicado

# Run with custom config directory
nix run github:olafkfreund/comunicado -- --config-dir ~/.config/comunicado-test
```

#### Option 2: Flake Installation
Add to your `flake.nix`:
```nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    comunicado.url = "github:olafkfreund/comunicado";
  };

  outputs = { self, nixpkgs, comunicado }:
    let
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages.${system};
    in {
      # Add to your system packages
      environment.systemPackages = [ 
        comunicado.packages.${system}.default 
      ];
    };
}
```

#### Option 3: System-Wide Installation (NixOS Configuration)
Add to your `/etc/nixos/configuration.nix`:
```nix
{ inputs, ... }: {
  imports = [ inputs.comunicado.nixosModules.default ];
  
  programs.comunicado.enable = true;
}
```

#### Development Environment
```bash
# Enter development shell
nix develop github:olafkfreund/comunicado

# Build from source
nix build github:olafkfreund/comunicado
```

### 📦 Arch Linux (AUR)

**For Arch Linux and Arch-based distributions (Manjaro, EndeavourOS, etc.)**

#### Using an AUR Helper (Recommended)
```bash
# Using paru (recommended)
paru -S comunicado

# Using yay
yay -S comunicado

# Using pamac (Manjaro)
pamac install comunicado
```

#### Manual AUR Installation
```bash
# Clone the AUR package
git clone https://aur.archlinux.org/comunicado.git
cd comunicado

# Build and install
makepkg -si

# Or build without installing
makepkg -s
sudo pacman -U comunicado-*.pkg.tar.zst
```

#### Dependencies
The package automatically installs required dependencies:
- `openssl`, `sqlite`, `dbus` (required)
- `gnupg`, `kitty`, `foot`, `wezterm` (optional, recommended)

### 🟦 Debian / Ubuntu

**For Debian, Ubuntu, and derivatives (Linux Mint, Pop!_OS, etc.)**

#### Prerequisites
```bash
# Update package lists
sudo apt update

# Install required dependencies
sudo apt install libssl3 libsqlite3-0 libdbus-1-3

# Install optional dependencies
sudo apt install gpg ca-certificates fonts-dejavu-core kitty
```

#### Installation
1. **Download the .deb package** from [GitHub Releases](https://github.com/olafkfreund/comunicado/releases)

2. **Install using dpkg:**
```bash
# Download (replace with actual release URL)
wget https://github.com/olafkfreund/comunicado/releases/download/v0.1.0/comunicado_0.1.0-1_amd64.deb

# Install
sudo dpkg -i comunicado_0.1.0-1_amd64.deb

# Fix dependencies if needed
sudo apt install -f
```

3. **Alternative: Install using apt:**
```bash
# Install with automatic dependency resolution
sudo apt install ./comunicado_0.1.0-1_amd64.deb
```

#### Building from Source (Advanced)
```bash
# Install build dependencies
sudo apt install debhelper-compat cargo rustc libssl-dev libsqlite3-dev pkg-config libdbus-1-dev

# Clone and build
git clone https://github.com/olafkfreund/comunicado.git
cd comunicado
dpkg-buildpackage -us -uc -b

# Install generated package
sudo dpkg -i ../comunicado_*.deb
```

### 🔴 Fedora / RHEL / CentOS

**For Fedora, RHEL 8+, CentOS Stream, Rocky Linux, AlmaLinux**

#### Prerequisites
```bash
# Fedora
sudo dnf install openssl sqlite dbus-libs

# RHEL/CentOS (requires EPEL)
sudo dnf install epel-release
sudo dnf install openssl sqlite dbus-libs

# Install optional dependencies
sudo dnf install gnupg2 ca-certificates dejavu-sans-fonts kitty
```

#### Installation
1. **Download the .rpm package** from [GitHub Releases](https://github.com/olafkfreund/comunicado/releases)

2. **Install using dnf/rpm:**
```bash
# Download (replace with actual release URL)
wget https://github.com/olafkfreund/comunicado/releases/download/v0.1.0/comunicado-0.1.0-1.fc39.x86_64.rpm

# Install with dnf (recommended - handles dependencies)
sudo dnf install comunicado-0.1.0-1.fc39.x86_64.rpm

# Or install with rpm (manual dependency management)
sudo rpm -ivh comunicado-0.1.0-1.fc39.x86_64.rpm
```

#### Building from Source (Advanced)
```bash
# Install build dependencies
sudo dnf install rust cargo gcc openssl-devel sqlite-devel pkgconfig dbus-devel rpm-build

# Build RPM
git clone https://github.com/olafkfreund/comunicado.git
cd comunicado
rpmbuild -ba comunicado.spec

# Install generated package
sudo rpm -ivh ~/rpmbuild/RPMS/*/comunicado-*.rpm
```

### 🦀 Universal Installation (Cargo)

**For any Linux distribution with Rust installed**

#### Prerequisites
```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Install system dependencies (varies by distribution)
# Debian/Ubuntu:
sudo apt install libssl-dev libsqlite3-dev pkg-config libdbus-1-dev

# Fedora:
sudo dnf install openssl-devel sqlite-devel pkgconfig dbus-devel

# Arch Linux:
sudo pacman -S openssl sqlite pkgconfig dbus
```

#### Installation
```bash
# Install from Git (latest development version)
cargo install --git https://github.com/olafkfreund/comunicado

# Install from crates.io (when published)
cargo install comunicado

# Install with all features
cargo install --git https://github.com/olafkfreund/comunicado --all-features

# Install with specific features only
cargo install --git https://github.com/olafkfreund/comunicado --no-default-features --features "notifications"
```

## Post-Installation Setup

### 1. Initial Configuration
```bash
# Run setup wizard (recommended for first-time users)
comunicado setup

# Or manually create config directory
mkdir -p ~/.config/comunicado
```

### 2. Terminal Compatibility Check
```bash
# Test terminal graphics support
comunicado --help | head -1
echo "If you see colors and formatting, your terminal is compatible!"
```

### 3. Verify Installation
```bash
# Check version
comunicado --version

# Test basic functionality
comunicado --help

# Run with verbose output to check for issues
comunicado --verbose
```

## Recommended Terminal Setup

For the best experience, use one of these terminals:

### 🐱 Kitty (Recommended)
```bash
# Install Kitty
# Arch: sudo pacman -S kitty
# Debian: sudo apt install kitty  
# Fedora: sudo dnf install kitty

# Enable graphics protocol
echo "Enable graphics protocol in ~/.config/kitty/kitty.conf:"
echo "allow_remote_control yes"
```

### 🦶 Foot (Wayland)
```bash
# Excellent for Wayland users
# Arch: sudo pacman -S foot
# Debian: sudo apt install foot
# Fedora: sudo dnf install foot
```

### 🚀 WezTerm (Cross-platform)
```bash
# Download from https://wezfurlong.org/wezterm/installation.html
# Or use package managers where available
```

## Troubleshooting

### Common Issues

#### 🔐 "Permission denied" errors
```bash
# Ensure binary is executable
chmod +x /usr/bin/comunicado

# Check file permissions
ls -la /usr/bin/comunicado
```

#### 📚 Missing dependencies
```bash
# Check system dependencies
ldd $(which comunicado) | grep "not found"

# Install missing libraries based on your distribution
```

#### 🐛 Database errors
```bash
# Reset databases if corrupted
rm -rf ~/.local/share/comunicado/*.db

# Run setup again
comunicado setup
```

#### 🎨 Display issues in terminal
```bash
# Check terminal capabilities
echo $TERM
echo $COLORTERM

# Try different terminal or update terminal configuration
```

### Getting Help

1. **Check the documentation:**
   ```bash
   man comunicado
   comunicado --help
   ```

2. **View plugin reference:**
   ```bash
   comunicado notes --help
   comunicado kde-connect --help
   ```

3. **Enable verbose logging:**
   ```bash
   comunicado --verbose
   ```

4. **Report issues:**
   - GitHub: https://github.com/olafkfreund/comunicado/issues
   - Include: OS version, terminal type, error messages

## Feature Configuration

### Enable/Disable Features
Build with custom features using Cargo:
```bash
# Minimal installation (no notifications, no KDE Connect)
cargo install --git https://github.com/olafkfreund/comunicado --no-default-features

# With notifications but no KDE Connect
cargo install --git https://github.com/olafkfreund/comunicado --no-default-features --features "notifications"

# Full featured installation
cargo install --git https://github.com/olafkfreund/comunicado --all-features
```

### Available Features
- `notifications` - Desktop notification support
- `kde-connect` - KDE Connect integration
- `webp-images` - WebP image format support
- `jpeg-images` - JPEG image format support
- `experimental-crypto` - Experimental GPG crypto features

## Next Steps

After installation, see:
- [Configuration Guide](docs/configuration.md) - Configure email accounts and calendar
- [CLI Reference](docs/cli-plugins-reference.md) - Complete command reference
- [Plugin Documentation](docs/plugin-architecture.md) - Available plugins and features

---

**Happy emailing in the terminal!** 🚀