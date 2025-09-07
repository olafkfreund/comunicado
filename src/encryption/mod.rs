//! Email Encryption Module
//!
//! Provides GPG encryption, decryption, and signing functionality for email messages.
//! Supports both GPGME (system GPG) and Sequoia-PGP (pure Rust) backends.

pub mod gpg;
pub mod manager;
pub mod types;
pub mod ui;

pub use gpg::{GpgBackend, GpgConfig, SequoiaGpgBackend, SystemGpgBackend};
pub use manager::EncryptionManager;
pub use types::{
    DecryptionStatus, EncryptionError, EncryptionResult, EncryptionStatus, KeyInfo, SignatureInfo,
};
pub use ui::EncryptionUI;

/// Re-exports for convenience
pub use sequoia_openpgp as sequoia;

#[cfg(test)]
mod tests;
