# AUR Submission Guide

> Instructions for submitting Comunicado to the Arch User Repository
> Package: `comunicado`
> Version: v0.1.0

## 📋 Pre-Submission Checklist

### ✅ Package Files Ready
- [x] `aur/PKGBUILD` - Complete build script
- [x] `aur/.SRCINFO` - AUR metadata file
- [x] All dependencies verified and documented
- [x] Optional dependencies listed with descriptions

### ✅ Package Validation
- [x] PKGBUILD follows Arch packaging guidelines
- [x] Package builds successfully with `makepkg -si`
- [x] No conflicts with existing packages
- [x] License properly specified (AGPL-3.0-only)

## 🚀 AUR Submission Process

### Step 1: AUR Account Setup
1. **Create AUR Account**: https://aur.archlinux.org/register/
2. **Upload SSH Key**: Add your public SSH key to AUR account
3. **Verify Access**: Test SSH connection to `aur@aur.archlinux.org`

### Step 2: Package Repository Setup
```bash
# Clone the AUR package repository (initially empty)
git clone ssh://aur@aur.archlinux.org/comunicado.git aur-comunicado
cd aur-comunicado

# Copy package files
cp ../aur/PKGBUILD .
cp ../aur/.SRCINFO .

# Initial commit
git add .
git commit -m "Initial import: comunicado 0.1.0-1

Modern TUI-based email and calendar client for terminal power users.

Features:
- Modern TUI interface with HTML email rendering
- OAuth2 support for Gmail, Outlook, and custom IMAP servers  
- CalDAV calendar integration with bidirectional sync
- Plugin system with Notes and KDE Connect plugins
- GPG encryption with Sequoia-PGP backend
- Performance optimized with modular build system

Upstream: https://github.com/olafkfreund/comunicado"

# Push to AUR
git push origin main
```

### Step 3: Package Testing
Before submission, verify the package works:

```bash
# Build and install
makepkg -si

# Test basic functionality
comunicado --version
comunicado --help

# Test setup wizard
comunicado setup

# Clean up test installation
sudo pacman -Rs comunicado
```

### Step 4: Update Package Files (if needed)

#### Update SHA256 Hashes
When the GitHub release is published, update the source hash:

```bash
# Calculate correct hash
curl -L https://github.com/olafkfreund/comunicado/archive/v0.1.0.tar.gz | sha256sum

# Update PKGBUILD
# Replace 'SKIP' with actual hash in sha256sums=()

# Regenerate .SRCINFO
makepkg --printsrcinfo > .SRCINFO

# Commit updates  
git add .
git commit -m "Update source hash for v0.1.0 release"
git push origin main
```

## 📦 Package Details

### Package Information
```
pkgname=comunicado
pkgver=0.1.0
pkgrel=1
pkgdesc="Modern TUI-based email and calendar client"  
arch=('x86_64' 'aarch64')
url="https://github.com/olafkfreund/comunicado"
license=('AGPL-3.0-only')
```

### Dependencies
**Required:**
- `openssl` - TLS/SSL support
- `sqlite` - Local database storage  
- `dbus` - System integration
- `gcc-libs` - Runtime libraries
- `glibc` - C library

**Build Dependencies:**
- `cargo` - Rust package manager
- `rust` - Rust compiler
- `pkg-config` - Library configuration
- `git` - Source code management

**Optional Dependencies:**
- `gnupg` - Email encryption and signing
- `kitty` - Recommended terminal with full graphics support
- `foot` - Wayland native terminal with good performance
- `wezterm` - Cross-platform terminal with advanced features
- `tmux` - Terminal multiplexer integration
- `ca-certificates` - Secure HTTPS connections
- `ttf-dejavu` - Recommended fonts for better display
- `libnotify` - Desktop notifications

## 🎯 AUR Package Guidelines Compliance

### ✅ Naming and Versioning
- Package name matches upstream: `comunicado`
- Version follows upstream releases
- Release number increments for packaging changes

### ✅ Dependencies  
- All runtime dependencies specified in `depends`
- Build dependencies in `makedepends`
- Optional features clearly documented in `optdepends`

### ✅ Build Process
- Uses upstream build system (Cargo)
- System libraries used where possible (no vendoring)
- Tests run during build (network tests skipped)
- Follows Rust packaging best practices

### ✅ Installation
- Binary installed to `/usr/bin/`
- Documentation installed to `/usr/share/doc/`
- Desktop entry for applications menu
- Man page for command-line reference
- Example configuration provided

### ✅ Metadata
- Complete package description
- Proper license specification
- Homepage and source URLs
- Architecture support documented

## 📊 Expected Package Statistics

### Build Time
- **First Build**: 10-15 minutes (many Rust dependencies)
- **Incremental**: 2-5 minutes (cached dependencies)
- **Parallel Jobs**: Supports `makepkg -j$(nproc)`

### Package Size
- **Built Package**: ~15-25MB compressed
- **Installed Size**: ~80-120MB (optimized release binary)
- **Source Download**: ~2-5MB (excluding Rust dependencies)

### Compatibility
- **Architecture**: x86_64, aarch64
- **Kernel**: Linux 5.4+
- **Terminal**: Modern terminal recommended (Kitty, Foot, WezTerm)

## 🚨 Common Issues and Solutions

### Build Failures
1. **Rust Version**: Ensure Rust 1.70+ available
2. **System Libraries**: Install all build dependencies
3. **Network Access**: Some dependencies require internet during build
4. **Disk Space**: Rust compilation requires significant space (~2GB)

### Runtime Issues
1. **Missing Libraries**: Check all dependencies installed
2. **Terminal Graphics**: Requires modern terminal for full features  
3. **Permissions**: May need additional permissions for keyring access
4. **Configuration**: First-time setup requires setup wizard

### AUR Submission Issues
1. **SSH Access**: Verify SSH key uploaded to AUR account
2. **Package Conflicts**: Ensure no conflicts with existing packages
3. **Licensing**: AGPL-3.0-only must be properly specified
4. **Source Verification**: Hash must match actual source

## 📞 Support and Maintenance

### Community Support
- **AUR Comments**: Monitor package comments for user issues
- **GitHub Issues**: Direct users to main repository for bugs
- **Wiki Updates**: Update Arch Wiki if needed

### Package Updates
- **Version Bumps**: Update when new releases are published
- **Dependency Changes**: Monitor for new or removed dependencies
- **Security Updates**: Respond promptly to security issues
- **Policy Changes**: Follow AUR packaging guideline updates

## 🎉 Post-Submission Success

### Immediate Actions
1. **Monitor Comments**: Watch for build issues from users
2. **Test Installation**: Verify package installs correctly
3. **Update Documentation**: Link to AUR package in README

### Long-term Maintenance
1. **Regular Updates**: Keep package current with releases
2. **Community Engagement**: Respond to user feedback
3. **Documentation**: Keep AUR package description updated
4. **Dependencies**: Monitor for upstream dependency changes

---

**The AUR package is ready for submission!** All files have been prepared following Arch packaging guidelines and best practices.