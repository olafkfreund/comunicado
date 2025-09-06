# Multi-stage Dockerfile for optimal container size and security
# Stage 1: Builder
FROM rust:1.70-slim AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libsqlite3-dev \
    libdbus-1-dev \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

# Create a non-root user for building
RUN useradd -m -u 1001 builder

# Set working directory
WORKDIR /app

# Copy dependency files first for better caching
COPY Cargo.toml Cargo.lock ./

# Create dummy source to cache dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm src/main.rs

# Copy the actual source code
COPY . .

# Force rebuild of our application (dependencies are cached)
RUN touch src/main.rs

# Build the application with all features
RUN cargo build --release --all-features

# Strip the binary to reduce size
RUN strip target/release/comunicado

# Stage 2: Runtime
FROM debian:bookworm-slim AS runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    libsqlite3-0 \
    libdbus-1-3 \
    && rm -rf /var/lib/apt/lists/* \
    && apt-get clean

# Create a non-root user for running the application
RUN useradd -m -u 1001 -s /bin/bash comunicado

# Create necessary directories
RUN mkdir -p /home/comunicado/.config/comunicado \
    && mkdir -p /home/comunicado/.local/share/comunicado \
    && mkdir -p /home/comunicado/.cache/comunicado \
    && chown -R comunicado:comunicado /home/comunicado

# Copy the binary from builder stage
COPY --from=builder /app/target/release/comunicado /usr/local/bin/comunicado

# Copy documentation and configuration examples
COPY --from=builder /app/README.md /usr/share/doc/comunicado/
COPY --from=builder /app/docs/ /usr/share/doc/comunicado/docs/

# Create default configuration
RUN cat > /home/comunicado/.config/comunicado/config.toml << 'EOF'
# Comunicado Container Configuration

[ui]
theme = "dark"
enable_animations = false  # Disabled in containers for performance
enable_mouse = false       # Terminal in container typically doesn't need mouse

[email]
database_path = "~/.local/share/comunicado/email.db"
cache_size_mb = 100

[calendar]
database_path = "~/.local/share/comunicado/calendar.db"
sync_interval_minutes = 30

[performance]
lazy_loading = true
background_sync = true
max_concurrent_connections = 3

[logging]
level = "info"
file = "~/.cache/comunicado/comunicado.log"

[plugins]
notes = { enabled = true }
kde_connect = { enabled = false }  # Disabled in containers
EOF

# Set ownership
RUN chown -R comunicado:comunicado /home/comunicado

# Switch to non-root user
USER comunicado
WORKDIR /home/comunicado

# Set environment variables for optimal container operation
ENV RUST_LOG=comunicado=info
ENV TERM=xterm-256color
ENV COMUNICADO_CONTAINER=true
ENV XDG_CONFIG_HOME=/home/comunicado/.config
ENV XDG_DATA_HOME=/home/comunicado/.local/share
ENV XDG_CACHE_HOME=/home/comunicado/.cache

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
  CMD comunicado --health-check || exit 1

# Document the port (if web interface is added later)
EXPOSE 8080

# Default command
ENTRYPOINT ["comunicado"]
CMD ["--help"]

# Labels for better container metadata
LABEL org.opencontainers.image.title="Comunicado"
LABEL org.opencontainers.image.description="Modern TUI-based email and calendar client"
LABEL org.opencontainers.image.url="https://github.com/olafkfreund/comunicado"
LABEL org.opencontainers.image.source="https://github.com/olafkfreund/comunicado"
LABEL org.opencontainers.image.vendor="Olaf K Freund"
LABEL org.opencontainers.image.licenses="AGPL-3.0-only"