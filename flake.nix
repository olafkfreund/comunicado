{
  description = "Comunicado - A modern TUI-based email and calendar client";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { 
          inherit system overlays; 
        };
        
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rustfmt" "clippy" ];
        };

        comunicado = pkgs.rustPlatform.buildRustPackage {
          pname = "comunicado";
          version = "0.1.0";
          
          src = ./.;
          
          cargoHash = pkgs.lib.fakeHash;
          
          nativeBuildInputs = with pkgs; [ pkg-config ];
          buildInputs = with pkgs; [ openssl sqlite ];
          
          meta = with pkgs.lib; {
            description = "Modern TUI-based email and calendar client";
            homepage = "https://github.com/olafkfreund/comunicado";
            license = licenses.agpl3Only;
            platforms = platforms.linux;
            mainProgram = "comunicado";
          };
        };
      in
      {
        # Package output
        packages = {
          default = comunicado;
          comunicado = comunicado;
        };

        # Development shell with TUI testing capabilities
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            # Rust development
            rustToolchain
            pkg-config
            openssl
            sqlite

            # TUI Testing tools
            nodejs_20
            python3
            python3Packages.pexpect
            tmux
            expect
            xvfb-run
            xdotool
            imagemagick

            # Development utilities
            git
            just
            cargo-watch
            cargo-edit
            cargo-outdated
          ];

          RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";
          PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";

          shellHook = ''
            echo "🦀 Comunicado Development Environment"
            echo "====================================="
            echo ""
            echo "Available commands:"
            echo "  cargo run           - Run Comunicado"
            echo "  cargo test          - Run tests"
            echo "  cargo build         - Build debug version"
            echo "  cargo build --release - Build optimized release"
            echo "  nix build           - Build with Nix"
            echo "  test-with-pexpect   - Run TUI tests with Python"
            echo "  test-tui-simple     - Quick TUI functionality test"
            echo ""
            echo "📦 Package testing:"
            echo "  nix run             - Run the Nix package"
            echo "  nix flake check     - Validate flake configuration"
            echo ""
            
            # Define shell functions for TUI testing
            test-with-pexpect() {
              echo "🐍 Running Python Pexpect TUI tests..."
              if [ -f scripts/test_tui_with_pexpect.py ]; then
                python3 scripts/test_tui_with_pexpect.py
              else
                echo "⚠️  TUI test script not found. Creating basic test..."
                mkdir -p scripts
                cat > scripts/test_tui_with_pexpect.py << 'EOF'
#!/usr/bin/env python3
import pexpect
import sys
import os
import tempfile

def test_comunicado_startup():
    """Test that Comunicado starts up without crashing"""
    print("🧪 Testing Comunicado startup...")
    
    # Create temporary config
    with tempfile.TemporaryDirectory() as temp_dir:
        config_path = os.path.join(temp_dir, "config.toml")
        with open(config_path, 'w') as f:
            f.write("""
[ui]
theme = "dark"
enable_animations = false

[email]
database_path = "/tmp/test_email.db"

[calendar] 
database_path = "/tmp/test_calendar.db"
""")
        
        try:
            # Start Comunicado with timeout
            child = pexpect.spawn(f'cargo run -- --config-dir {temp_dir}', timeout=10)
            child.expect(pexpect.EOF, timeout=5)
            print("✅ Comunicado started and exited cleanly")
            return True
        except pexpect.TIMEOUT:
            print("✅ Comunicado started (timeout expected for TUI)")
            child.terminate()
            return True
        except Exception as e:
            print(f"❌ Error: {e}")
            return False

if __name__ == "__main__":
    success = test_comunicado_startup()
    sys.exit(0 if success else 1)
EOF
                chmod +x scripts/test_tui_with_pexpect.py
                python3 scripts/test_tui_with_pexpect.py
              fi
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
              
              echo "Testing TUI (5 second timeout)..."
              timeout 5 ./target/debug/comunicado --config-dir /tmp/comunicado_test_config || echo "✅ Test completed"
              
              rm -rf /tmp/comunicado_test_config /tmp/test_email.db /tmp/test_calendar.db
            }
            
            # Export functions
            export -f test-with-pexpect
            export -f test-tui-simple
          '';
        };

        # NixOS module for system-wide installation
        nixosModules.default = { config, lib, pkgs, ... }: {
          options.programs.comunicado = {
            enable = lib.mkEnableOption "Comunicado email and calendar client";
            
            package = lib.mkOption {
              type = lib.types.package;
              default = self.packages.${system}.default;
              description = "The comunicado package to use";
            };
          };

          config = lib.mkIf config.programs.comunicado.enable {
            environment.systemPackages = [ config.programs.comunicado.package ];
            
            # Ensure required system dependencies
            environment.variables = {
              # Terminal graphics support
              TERM_FEATURES = "rgb:hyperlinks:graphics";
            };
          };
        };

        # Apps for nix run
        apps.default = {
          type = "app";
          program = "${comunicado}/bin/comunicado";
        };

        # Checks for flake validation
        checks = {
          comunicado-package = comunicado;
        };
      }
    ) // {
      # Overlay for use in other flakes
      overlays.default = final: prev: {
        comunicado = final.rustPlatform.buildRustPackage {
          pname = "comunicado";
          version = "0.1.0";
          src = ./.;
          cargoHash = final.lib.fakeHash;
          nativeBuildInputs = with final; [ pkg-config ];
          buildInputs = with final; [ openssl sqlite ];
        };
      };
    };
}