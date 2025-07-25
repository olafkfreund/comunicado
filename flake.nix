{
  description = "Comunicado - A modern TUI-based email and calendar client";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    devenv.url = "github:cachix/devenv";
  };

  outputs = inputs@{ flake-parts, nixpkgs, devenv, rust-overlay, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      imports = [
        inputs.devenv.flakeModule
      ];
      systems = [ "x86_64-linux" "aarch64-linux" "aarch64-darwin" "x86_64-darwin" ];

      perSystem = { config, self', inputs', pkgs, system, ... }: let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
      in {
        # Development environment
        devenv.shells.default = {
          packages = with pkgs; [
            # Rust toolchain
            (rust-bin.stable.latest.default.override {
              extensions = [ "rust-src" "rustfmt" "clippy" ];
            })
            
            # Build tools
            pkg-config
            openssl
            sqlite

            # Development tools
            git
            just
            bacon
          ];

          env = {
            RUST_BACKTRACE = "1";
            PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";
          };

          enterShell = ''
            echo "🚀 Welcome to Comunicado development environment!"
            echo "📧 Modern TUI email and calendar client"
            echo ""
            echo "Available commands:"
            echo "  cargo build    - Build the project"
            echo "  cargo test     - Run tests"
            echo "  cargo run      - Run the application"
            echo "  just --list    - Show available just commands"
          '';
        };

        # Package for building/installing
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "comunicado";
          version = "0.1.0";
          
          src = ./.;
          
          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          nativeBuildInputs = with pkgs; [
            pkg-config
          ];

          buildInputs = with pkgs; [
            openssl
            sqlite
          ];

          meta = with pkgs.lib; {
            description = "A modern TUI-based email and calendar client";
            homepage = "https://github.com/your-username/comunicado";
            license = with licenses; [ mit asl20 ];
            maintainers = [ ];
          };
        };
      };
    };
}