//! Types and data structures for email encryption

use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

/// Result type for encryption operations
pub type EncryptionResult<T> = Result<T, EncryptionError>;

/// Errors that can occur during encryption operations
#[derive(Error, Debug)]
pub enum EncryptionError {
    #[error("GPG error: {0}")]
    GpgError(String),

    #[error("Key not found: {0}")]
    KeyNotFound(String),

    #[error("Invalid key: {0}")]
    InvalidKey(String),

    #[error("Decryption failed: {0}")]
    DecryptionFailed(String),

    #[error("Encryption failed: {0}")]
    EncryptionFailed(String),

    #[error("Signing failed: {0}")]
    SigningFailed(String),

    #[error("Signature verification failed: {0}")]
    SignatureVerificationFailed(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("GPGME error: {0}")]
    GpgmeError(String),

    #[error("Sequoia error: {0}")]
    SequoiaError(String),

    #[error("Anyhow error: {0}")]
    AnyhowError(#[from] anyhow::Error),
}

/// Information about a GPG key
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyInfo {
    /// Key ID (short or long format)
    pub key_id: String,

    /// Key fingerprint
    pub fingerprint: String,

    /// User IDs associated with the key
    pub user_ids: Vec<String>,

    /// Primary email address
    pub email: Option<String>,

    /// Key creation date
    pub creation_date: Option<chrono::DateTime<chrono::Utc>>,

    /// Key expiration date
    pub expiration_date: Option<chrono::DateTime<chrono::Utc>>,

    /// Whether this is a secret key
    pub is_secret: bool,

    /// Whether the key is expired
    pub is_expired: bool,

    /// Whether the key is revoked  
    pub is_revoked: bool,

    /// Key trust level
    pub trust_level: TrustLevel,

    /// Key capabilities (encrypt, sign, certify, authenticate)
    pub capabilities: KeyCapabilities,
}

/// Trust level of a GPG key
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TrustLevel {
    Unknown,
    Never,
    Marginal,
    Full,
    Ultimate,
}

/// Capabilities of a GPG key
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyCapabilities {
    pub can_encrypt: bool,
    pub can_sign: bool,
    pub can_certify: bool,
    pub can_authenticate: bool,
}

/// Information about a digital signature
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureInfo {
    /// Key ID of the signer
    pub key_id: String,

    /// Email address of the signer
    pub signer_email: Option<String>,

    /// Name of the signer
    pub signer_name: Option<String>,

    /// Signature creation time
    pub creation_time: Option<chrono::DateTime<chrono::Utc>>,

    /// Signature validity
    pub validity: SignatureValidity,

    /// Trust level of the signing key
    pub trust_level: TrustLevel,
}

/// Validity status of a signature
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SignatureValidity {
    Valid,
    Invalid,
    KeyNotFound,
    KeyExpired,
    KeyRevoked,
    BadSignature,
    Unknown,
}

/// Status of encryption for an email message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionStatus {
    /// Whether the message is encrypted
    pub is_encrypted: bool,

    /// Recipients for whom the message was encrypted
    pub encrypted_for: Vec<String>,

    /// Encryption algorithm used
    pub algorithm: Option<String>,

    /// Whether decryption was successful
    pub decryption_successful: bool,

    /// Error message if decryption failed
    pub decryption_error: Option<String>,
}

/// Status of signatures for an email message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecryptionStatus {
    /// Whether the message has signatures
    pub is_signed: bool,

    /// List of signatures found
    pub signatures: Vec<SignatureInfo>,

    /// Overall signature validity
    pub overall_validity: SignatureValidity,
}

/// Combined encryption and signature status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageSecurityStatus {
    pub encryption: EncryptionStatus,
    pub signatures: DecryptionStatus,
}

/// Email message content with encryption/signature information
#[derive(Debug, Clone)]
pub struct SecureEmailContent {
    /// Original encrypted/signed content
    pub raw_content: String,

    /// Decrypted content (if applicable)
    pub decrypted_content: Option<String>,

    /// Security status
    pub security_status: MessageSecurityStatus,
}

/// Configuration for encryption operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionConfig {
    /// Default key for signing (key ID or email)
    pub default_signing_key: Option<String>,

    /// Whether to always sign outgoing emails
    pub always_sign: bool,

    /// Whether to encrypt emails when possible
    pub auto_encrypt: bool,

    /// GPG home directory (optional, uses system default if None)
    pub gpg_home: Option<String>,

    /// Preferred encryption algorithm
    pub preferred_cipher: Option<String>,

    /// Preferred compression algorithm
    pub preferred_compression: Option<String>,

    /// Key server for key retrieval
    pub key_server: Option<String>,

    /// Timeout for GPG operations (in seconds)
    pub operation_timeout: u64,
}

impl Default for EncryptionConfig {
    fn default() -> Self {
        Self {
            default_signing_key: None,
            always_sign: false,
            auto_encrypt: false,
            gpg_home: None,
            preferred_cipher: None,
            preferred_compression: None,
            key_server: Some("keys.openpgp.org".to_string()),
            operation_timeout: 30,
        }
    }
}

impl fmt::Display for TrustLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TrustLevel::Unknown => write!(f, "Unknown"),
            TrustLevel::Never => write!(f, "Never"),
            TrustLevel::Marginal => write!(f, "Marginal"),
            TrustLevel::Full => write!(f, "Full"),
            TrustLevel::Ultimate => write!(f, "Ultimate"),
        }
    }
}

impl fmt::Display for SignatureValidity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SignatureValidity::Valid => write!(f, "Valid"),
            SignatureValidity::Invalid => write!(f, "Invalid"),
            SignatureValidity::KeyNotFound => write!(f, "Key Not Found"),
            SignatureValidity::KeyExpired => write!(f, "Key Expired"),
            SignatureValidity::KeyRevoked => write!(f, "Key Revoked"),
            SignatureValidity::BadSignature => write!(f, "Bad Signature"),
            SignatureValidity::Unknown => write!(f, "Unknown"),
        }
    }
}

impl KeyInfo {
    /// Check if the key is usable for encryption
    pub fn can_encrypt(&self) -> bool {
        self.capabilities.can_encrypt && !self.is_expired && !self.is_revoked
    }

    /// Check if the key is usable for signing
    pub fn can_sign(&self) -> bool {
        self.capabilities.can_sign && !self.is_expired && !self.is_revoked
    }

    /// Get the primary email address or first user ID
    pub fn primary_identity(&self) -> Option<&str> {
        self.email
            .as_deref()
            .or_else(|| self.user_ids.first().map(|s| s.as_str()))
    }
}

impl EncryptionStatus {
    /// Create a new unencrypted status
    pub fn unencrypted() -> Self {
        Self {
            is_encrypted: false,
            encrypted_for: Vec::new(),
            algorithm: None,
            decryption_successful: false,
            decryption_error: None,
        }
    }

    /// Create a new encrypted status with successful decryption
    pub fn encrypted_and_decrypted(recipients: Vec<String>, algorithm: Option<String>) -> Self {
        Self {
            is_encrypted: true,
            encrypted_for: recipients,
            algorithm,
            decryption_successful: true,
            decryption_error: None,
        }
    }

    /// Create a new encrypted status with failed decryption
    pub fn encrypted_with_error(recipients: Vec<String>, error: String) -> Self {
        Self {
            is_encrypted: true,
            encrypted_for: recipients,
            algorithm: None,
            decryption_successful: false,
            decryption_error: Some(error),
        }
    }
}

impl DecryptionStatus {
    /// Create a new unsigned status
    pub fn unsigned() -> Self {
        Self {
            is_signed: false,
            signatures: Vec::new(),
            overall_validity: SignatureValidity::Unknown,
        }
    }

    /// Create a new signed status
    pub fn signed(signatures: Vec<SignatureInfo>) -> Self {
        let overall_validity = if signatures.is_empty() {
            SignatureValidity::Unknown
        } else if signatures
            .iter()
            .all(|s| s.validity == SignatureValidity::Valid)
        {
            SignatureValidity::Valid
        } else if signatures
            .iter()
            .any(|s| s.validity == SignatureValidity::Valid)
        {
            SignatureValidity::Valid // At least one valid signature
        } else {
            SignatureValidity::Invalid
        };

        Self {
            is_signed: true,
            signatures,
            overall_validity,
        }
    }
}

impl MessageSecurityStatus {
    /// Create a status for an unencrypted, unsigned message
    pub fn none() -> Self {
        Self {
            encryption: EncryptionStatus::unencrypted(),
            signatures: DecryptionStatus::unsigned(),
        }
    }

    /// Check if the message has any security features
    pub fn has_security(&self) -> bool {
        self.encryption.is_encrypted || self.signatures.is_signed
    }

    /// Get a summary description of the security status
    pub fn summary(&self) -> String {
        match (self.encryption.is_encrypted, self.signatures.is_signed) {
            (true, true) => {
                if self.encryption.decryption_successful
                    && self.signatures.overall_validity == SignatureValidity::Valid
                {
                    "Encrypted and signed ✓".to_string()
                } else {
                    "Encrypted and signed (with issues)".to_string()
                }
            }
            (true, false) => {
                if self.encryption.decryption_successful {
                    "Encrypted ✓".to_string()
                } else {
                    "Encrypted (decryption failed)".to_string()
                }
            }
            (false, true) => {
                if self.signatures.overall_validity == SignatureValidity::Valid {
                    "Signed ✓".to_string()
                } else {
                    "Signed (invalid signature)".to_string()
                }
            }
            (false, false) => "No encryption or signatures".to_string(),
        }
    }
}
