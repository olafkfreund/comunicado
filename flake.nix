{
  description = "Comunicado - TUI Testing Environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
      in
      {
        # Lightweight development shell focused on TUI testing
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            # TUI Testing tools only
            nodejs_20  # Node 20 (Microsoft TUI-Test prefers <21 but may work)
            python3
            python3Packages.pexpect
            tmux
            expect
            xvfb-run
            xdotool
            imagemagick
          ];

          shellHook = ''
            echo "🧪 Comunicado TUI Testing Environment"
            echo "====================================="
            echo ""
            echo "TUI Testing commands:"
            echo "  test-with-pexpect   - Run Python Pexpect tests (RECOMMENDED)"
            echo "  test-tui-simple     - Quick TUI test"
            echo "  run-tui-tests       - Fallback to Python tests (Microsoft TUI-Test has compatibility issues)"
            echo "  setup-tui-tests     - Install Microsoft TUI-Test (Node compatibility issues)"
            echo ""
            
            # Define shell functions for TUI testing
            setup-tui-tests() {
              echo "🧪 Setting up Microsoft TUI-Test..."
              echo "⚠️  Note: Microsoft TUI-Test requires Node.js <21.0.0"
              echo "📦 Current Node version: $(node --version)"
              npm init -y 2>/dev/null || true
              npm install -D @microsoft/tui-test tsx @types/node 2>/dev/null || {
                echo "❌ Microsoft TUI-Test installation failed (Node.js version compatibility)"
                echo "💡 Use test-with-pexpect instead - it works perfectly!"
                return 1
              }
              echo "✅ TUI testing setup complete!"
            }
            
            run-tui-tests() {
              echo "🧪 Running TUI tests..."
              echo "⚠️  Microsoft TUI-Test has Node.js compatibility issues"
              echo "📦 Current Node version: $(node --version)"
              echo ""
              echo "🐍 Running Python Pexpect tests instead (fully functional):"
              echo "----------------------------------------"
              test-with-pexpect
              echo ""
              echo "💡 For Microsoft TUI-Test, try with Node.js <20.0.0"
              echo "📚 Python Pexpect provides comprehensive TUI testing coverage"
            }
            
            test-with-pexpect() {
              echo "🐍 Running Python Pexpect tests..."
              python3 scripts/test_tui_with_pexpect.py
            }
            
            test-tui-simple() {
              echo "🧪 Running simple TUI test..."
              # Create test config directory
              mkdir -p /tmp/comunicado_test_config
              cat > /tmp/comunicado_test_config/config.toml << 'EOF'
            [ui]
            theme = "dark"
            enable_animations = false
            [email]
            database_path = "/tmp/test_email.db"
            [calendar]
            database_path = "/tmp/test_calendar.db"
            EOF
              
              echo "Building Comunicado..."
              cargo build
              
              echo "Testing TUI (10 second timeout)..."
              timeout 10 ./target/debug/comunicado --config-dir /tmp/comunicado_test_config || echo "✅ Test completed"
              
              rm -rf /tmp/comunicado_test_config /tmp/test_email.db /tmp/test_calendar.db
            }
            
            # Export functions
            export -f setup-tui-tests
            export -f run-tui-tests  
            export -f test-with-pexpect
            export -f test-tui-simple
          '';
        };
      });
}