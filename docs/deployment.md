# Comunicado Deployment Guide

This guide covers various deployment options for Comunicado, from local installation to production container deployment.

## Table of Contents

1. [Local Installation](#local-installation)
2. [Package Managers](#package-managers)
3. [Container Deployment](#container-deployment)
4. [Development Environment](#development-environment)
5. [Production Considerations](#production-considerations)
6. [CI/CD Pipeline](#cicd-pipeline)

## Local Installation

### From Source

```bash
# Clone the repository
git clone https://github.com/olafkfreund/comunicado.git
cd comunicado

# Install system dependencies
# Ubuntu/Debian
sudo apt-get install libssl-dev libsqlite3-dev pkg-config libdbus-1-dev

# Fedora/RHEL
sudo dnf install openssl-devel sqlite-devel pkgconf dbus-devel

# macOS
brew install openssl sqlite pkg-config

# Build and install
cargo build --release --all-features
sudo cp target/release/comunicado /usr/local/bin/
```

### From Cargo

```bash
cargo install comunicado --all-features
```

## Package Managers

### Arch Linux (AUR)

```bash
yay -S comunicado
# or
paru -S comunicado
```

### Fedora/RHEL

```bash
# Download the latest RPM from GitHub releases
sudo dnf install comunicado-0.1.0-1.x86_64.rpm
```

### Ubuntu/Debian

```bash
# Download the latest DEB from GitHub releases
sudo dpkg -i comunicado_0.1.0_amd64.deb
sudo apt-get install -f  # Fix dependencies if needed
```

### NixOS

```nix
# Add to your configuration.nix or home.nix
environment.systemPackages = with pkgs; [
  comunicado
];
```

### Homebrew (macOS)

```bash
brew tap olafkfreund/comunicado
brew install comunicado
```

## Container Deployment

### Using Docker

#### Quick Start

```bash
# Pull and run the latest version
docker run -it --rm \
  -v comunicado_config:/home/comunicado/.config \
  -v comunicado_data:/home/comunicado/.local/share \
  ghcr.io/olafkfreund/comunicado:latest
```

#### With Persistent Data

```bash
# Create directories for data persistence
mkdir -p ~/.config/comunicado
mkdir -p ~/.local/share/comunicado

# Run with volume mounts
docker run -it --rm \
  -v ~/.config/comunicado:/home/comunicado/.config/comunicado \
  -v ~/.local/share/comunicado:/home/comunicado/.local/share/comunicado \
  ghcr.io/olafkfreund/comunicado:latest
```

### Using Docker Compose

#### Production Setup

```yaml
# docker-compose.prod.yml
version: '3.8'

services:
  comunicado:
    image: ghcr.io/olafkfreund/comunicado:latest
    container_name: comunicado
    restart: unless-stopped
    tty: true
    stdin_open: true
    
    environment:
      - RUST_LOG=comunicado=info
      - TERM=xterm-256color
      
    volumes:
      - ./config:/home/comunicado/.config/comunicado
      - comunicado_data:/home/comunicado/.local/share/comunicado
      - comunicado_cache:/home/comunicado/.cache/comunicado
      
    deploy:
      resources:
        limits:
          memory: 512M
          cpus: '1.0'

volumes:
  comunicado_data:
  comunicado_cache:
```

```bash
# Deploy
docker-compose -f docker-compose.prod.yml up -d

# Access the container
docker-compose -f docker-compose.prod.yml exec comunicado bash
```

### Kubernetes Deployment

```yaml
# comunicado-deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: comunicado
  labels:
    app: comunicado
spec:
  replicas: 1
  selector:
    matchLabels:
      app: comunicado
  template:
    metadata:
      labels:
        app: comunicado
    spec:
      containers:
      - name: comunicado
        image: ghcr.io/olafkfreund/comunicado:latest
        resources:
          limits:
            memory: "512Mi"
            cpu: "1000m"
          requests:
            memory: "128Mi"
            cpu: "500m"
        env:
        - name: RUST_LOG
          value: "comunicado=info"
        - name: TERM
          value: "xterm-256color"
        volumeMounts:
        - name: config-volume
          mountPath: /home/comunicado/.config/comunicado
        - name: data-volume
          mountPath: /home/comunicado/.local/share/comunicado
        stdin: true
        tty: true
      volumes:
      - name: config-volume
        configMap:
          name: comunicado-config
      - name: data-volume
        persistentVolumeClaim:
          claimName: comunicado-data-pvc

---
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: comunicado-data-pvc
spec:
  accessModes:
    - ReadWriteOnce
  resources:
    requests:
      storage: 10Gi

---
apiVersion: v1
kind: ConfigMap
metadata:
  name: comunicado-config
data:
  config.toml: |
    [ui]
    theme = "dark"
    enable_animations = false
    
    [email]
    database_path = "~/.local/share/comunicado/email.db"
    
    [calendar]
    database_path = "~/.local/share/comunicado/calendar.db"
```

```bash
kubectl apply -f comunicado-deployment.yaml
kubectl exec -it deployment/comunicado -- bash
```

## Development Environment

### Using Docker Compose

```bash
# Start development environment with hot reload
docker-compose --profile dev up -d

# Access development container
docker-compose exec comunicado-dev bash

# Run tests in development container
docker-compose exec comunicado-dev cargo test

# Watch for changes
docker-compose exec comunicado-dev cargo watch -x "run --all-features"
```

### Local Development with Mock Services

```bash
# Start mock email and calendar servers
docker-compose --profile testing up -d mailhog radicale

# Configure Comunicado to use mock servers
cat > ~/.config/comunicado/config.toml << EOF
[email.servers.test]
imap_server = "localhost"
imap_port = 1143
smtp_server = "localhost"  
smtp_port = 1025

[calendar.servers.test]
caldav_url = "http://test:password@localhost:5232/"
EOF

# Access mock services
# MailHog UI: http://localhost:8025
# Radicale CalDAV: http://localhost:5232
```

## Production Considerations

### Security

1. **Container Security**
   - Use non-root user (already configured)
   - Keep base images updated
   - Scan for vulnerabilities
   - Use read-only file systems where possible

2. **Network Security**
   - Use TLS for all email connections
   - Validate SSL certificates
   - Consider using a reverse proxy

3. **Data Protection**
   - Encrypt data at rest
   - Secure credential storage
   - Regular backups of configuration and data

### Performance Optimization

1. **Resource Allocation**
   ```yaml
   deploy:
     resources:
       limits:
         memory: 1G        # Adjust based on usage
         cpus: '2.0'       # Scale for concurrent operations
       reservations:
         memory: 256M
         cpus: '1.0'
   ```

2. **Volume Configuration**
   - Use persistent volumes for data
   - Consider SSD storage for databases
   - Separate cache and data volumes

3. **Environment Tuning**
   ```bash
   # Environment variables for production
   RUST_LOG=comunicado=warn              # Reduce log verbosity
   COMUNICADO_CACHE_SIZE=500             # Adjust cache size (MB)
   COMUNICADO_MAX_CONNECTIONS=10         # Limit concurrent connections
   COMUNICADO_SYNC_INTERVAL=900          # Sync every 15 minutes
   ```

### Monitoring and Logging

1. **Health Checks**
   ```yaml
   healthcheck:
     test: ["CMD", "comunicado", "--health-check"]
     interval: 30s
     timeout: 10s
     retries: 3
     start_period: 60s
   ```

2. **Log Management**
   ```yaml
   logging:
     driver: "json-file"
     options:
       max-size: "10m"
       max-file: "3"
   ```

3. **Metrics Collection**
   - CPU and memory usage
   - Email sync performance
   - Connection health
   - Error rates

### Backup Strategy

1. **Data Backup**
   ```bash
   # Backup volumes
   docker run --rm \
     -v comunicado_data:/data \
     -v $(pwd):/backup \
     alpine tar czf /backup/comunicado-data-$(date +%Y%m%d).tar.gz -C /data .
   ```

2. **Configuration Backup**
   ```bash
   # Backup configuration
   tar czf comunicado-config-$(date +%Y%m%d).tar.gz \
     ~/.config/comunicado/
   ```

## CI/CD Pipeline

The project includes automated CI/CD pipelines that:

### Build and Test (`build-and-test.yml`)
- **Triggers**: Push to main/develop, Pull requests
- **Actions**:
  - Multi-platform testing (Linux, macOS, Windows)
  - MSRV (Minimum Supported Rust Version) validation
  - Code formatting and linting
  - Security auditing
  - Code coverage reporting
  - Performance benchmarking

### Release and Deploy (`release.yml`)
- **Triggers**: Git tags (v*)
- **Actions**:
  - Create GitHub release with changelog
  - Build multi-architecture binaries
  - Generate distribution packages (DEB, RPM, AppImage)
  - Build and push container images
  - Publish to Cargo registry
  - Update AUR package

### Manual Deployment

```bash
# Tag a new release
git tag -a v0.2.0 -m "Release version 0.2.0"
git push origin v0.2.0

# This triggers the release pipeline automatically
# Monitor at: https://github.com/olafkfreund/comunicado/actions
```

## Troubleshooting

### Common Issues

1. **TUI Not Displaying Properly**
   ```bash
   # Set correct terminal type
   export TERM=xterm-256color
   
   # For containers
   docker run -it -e TERM=xterm-256color comunicado
   ```

2. **Permission Issues**
   ```bash
   # Fix data directory permissions
   sudo chown -R $(id -u):$(id -g) ~/.local/share/comunicado
   ```

3. **Container Networking**
   ```bash
   # Check container network connectivity
   docker exec comunicado ping gmail.com
   
   # Check DNS resolution
   docker exec comunicado nslookup imap.gmail.com
   ```

### Debug Mode

```bash
# Enable debug logging
export RUST_LOG=comunicado=debug

# Or for containers
docker run -e RUST_LOG=comunicado=debug -it comunicado
```

### Getting Support

- **GitHub Issues**: [Report bugs and feature requests](https://github.com/olafkfreund/comunicado/issues)
- **Discussions**: [Community support](https://github.com/olafkfreund/comunicado/discussions)
- **Documentation**: [Full documentation](https://github.com/olafkfreund/comunicado/tree/main/docs)

---

For additional deployment scenarios or custom configurations, please refer to the specific documentation in the `docs/` directory or create an issue for support.