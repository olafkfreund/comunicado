# Comunicado Development Commands

# Default recipe - show available commands
default:
    @just --list

# Build the project
build:
    cargo build

# Build for release
build-release:
    cargo build --release

# Run the application
run:
    cargo run

# Run tests
test:
    cargo test

# Run tests with output
test-verbose:
    cargo test -- --nocapture

# Run clippy linter
lint:
    cargo clippy -- -D warnings

# Format code
fmt:
    cargo fmt

# Check formatting
fmt-check:
    cargo fmt --check

# Run all checks (format, lint, test)
check: fmt-check lint test

# Clean build artifacts
clean:
    cargo clean

# Watch for changes and rebuild
watch:
    bacon

# Generate documentation
docs:
    cargo doc --open

# Install the application
install:
    nix profile install .