{ lib
, rustPlatform
, fetchFromGitHub
, pkg-config
, openssl
, sqlite
, stdenv
, darwin
}:

rustPlatform.buildRustPackage rec {
  pname = "comunicado";
  version = "0.1.0";

  src = ./.;

  cargoHash = lib.fakeHash; # Will be updated after first build

  nativeBuildInputs = [
    pkg-config
  ];

  buildInputs = [
    openssl
    sqlite
  ] ++ lib.optionals stdenv.isDarwin [
    darwin.apple_sdk.frameworks.CoreFoundation
    darwin.apple_sdk.frameworks.Security
    darwin.apple_sdk.frameworks.SystemConfiguration
  ];

  # Enable all default features including notifications and experimental crypto
  buildFeatures = [ "default" ];

  # Skip tests that require network access or specific terminal environments
  checkFlags = [
    "--skip=test_imap_connection"
    "--skip=test_oauth_flow" 
    "--skip=test_caldav_sync"
  ];

  meta = with lib; {
    description = "A modern TUI-based email and calendar client for terminal power users";
    longDescription = ''
      Comunicado is a modern terminal-based email and calendar client that provides
      native HTML email rendering, integrated CalDAV calendar functionality, and 
      multi-provider OAuth2 support. Built for terminal power users, privacy-conscious
      developers, and system administrators who want rich email and calendar features
      without leaving their terminal environment.

      Key features:
      - Modern TUI interface with ratatui
      - HTML email rendering with images and animations
      - OAuth2 support for Gmail, Outlook, and other providers
      - CalDAV calendar integration
      - Plugin architecture with Notes and KDE Connect plugins
      - Maildir support for local email storage
      - Advanced search and email threading
      - Desktop notifications and keyboard customization
    '';
    homepage = "https://github.com/olafkfreund/comunicado";
    license = licenses.agpl3Only;
    maintainers = with maintainers; [ ]; # Add maintainer here when submitting to nixpkgs
    platforms = platforms.linux ++ platforms.darwin;
    mainProgram = "comunicado";
  };
}