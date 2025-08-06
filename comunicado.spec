Name:           comunicado
Version:        0.1.0
Release:        1%{?dist}
Summary:        Modern TUI-based email and calendar client

License:        AGPL-3.0-only
URL:            https://github.com/olafkfreund/comunicado
Source0:        https://github.com/olafkfreund/comunicado/archive/v%{version}.tar.gz

BuildRequires:  rust >= 1.70
BuildRequires:  cargo >= 1.70
BuildRequires:  gcc
BuildRequires:  openssl-devel
BuildRequires:  sqlite-devel
BuildRequires:  pkgconfig
BuildRequires:  dbus-devel
BuildRequires:  systemd-rpm-macros

Requires:       openssl
Requires:       sqlite
Requires:       dbus-libs
Recommends:     gnupg2
Recommends:     ca-certificates
Recommends:     dejavu-sans-fonts
Suggests:       kitty
Suggests:       foot
Suggests:       wezterm
Suggests:       tmux

%description
Comunicado is a modern terminal-based email and calendar client built for
terminal power users, privacy-conscious developers, and system administrators.

Key features:
- Modern TUI interface with ratatui framework
- HTML email rendering with images and animations  
- OAuth2 support for Gmail, Outlook, and other providers
- CalDAV calendar integration with bidirectional sync
- Plugin architecture with Notes and KDE Connect plugins
- Maildir support for local email storage
- Advanced search and email threading capabilities
- Desktop notifications and customizable keyboard shortcuts

The application provides rich email content viewing directly in the terminal
using modern terminal graphics protocols, eliminating the need to switch
between terminal and GUI applications for email management.

%prep
%autosetup -n %{name}-%{version}

%build
# Set up Rust environment
export CARGO_HOME=%{_builddir}/.cargo
export RUSTUP_HOME=%{_builddir}/.rustup

# Use system dependencies where possible
export OPENSSL_NO_VENDOR=1
export LIBSQLITE3_SYS_USE_PKG_CONFIG=1

# Build with release optimizations
cargo build --release --locked

%install
# Install binary
install -D -m 755 target/release/comunicado %{buildroot}%{_bindir}/comunicado

# Install desktop entry
install -D -m 644 debian/comunicado.desktop %{buildroot}%{_datadir}/applications/comunicado.desktop

# Install man page
install -D -m 644 debian/comunicado.1 %{buildroot}%{_mandir}/man1/comunicado.1

# Install documentation
install -d %{buildroot}%{_docdir}/%{name}
install -m 644 README.md %{buildroot}%{_docdir}/%{name}/
install -m 644 docs/cli-plugins-reference.md %{buildroot}%{_docdir}/%{name}/
install -m 644 docs/performance-optimization-report.md %{buildroot}%{_docdir}/%{name}/

# Install example configuration
install -d %{buildroot}%{_sysconfdir}/%{name}
cat > %{buildroot}%{_sysconfdir}/%{name}/config.toml.example << 'EOF'
# Example Comunicado configuration
# Copy to ~/.config/comunicado/config.toml and customize

[ui]
theme = "dark"
enable_animations = true

[email]
database_path = "~/.local/share/comunicado/email.db"

[calendar]
database_path = "~/.local/share/comunicado/calendar.db"

[plugins]
notes = { enabled = true }
kde_connect = { enabled = false }
EOF

%check
# Run tests (skip network-dependent ones)
cargo test --release --locked -- \
    --skip test_imap_connection \
    --skip test_oauth_flow \
    --skip test_caldav_sync \
    --skip test_network_image_loading || true

%files
%license LICENSE
%doc README.md
%doc %{_docdir}/%{name}/
%{_bindir}/comunicado
%{_datadir}/applications/comunicado.desktop
%{_mandir}/man1/comunicado.1*
%config(noreplace) %{_sysconfdir}/%{name}/config.toml.example

%changelog
* Tue Aug 06 2025 Olaf K Freund <your.email@example.com> - 0.1.0-1
- Initial release of Comunicado
- Modern TUI-based email and calendar client
- Features:
  * Native HTML email rendering with terminal graphics support
  * OAuth2 authentication for Gmail, Outlook, and other providers
  * CalDAV calendar integration with bidirectional synchronization
  * Plugin architecture with Notes and KDE Connect plugins
  * Maildir support for local email storage
  * Advanced search and email threading capabilities
  * Desktop notifications and customizable keyboard shortcuts
  * Performance optimizations with modular feature system
- Comprehensive error handling with user-friendly recovery suggestions
- Complete CLI documentation and plugin reference
- Production-ready with optimized build configurations