//! Comprehensive security hardening and protection system

pub mod authentication;
pub mod authorization;
pub mod encryption;
pub mod input_validation;
pub mod network_security;
pub mod audit_logging;
pub mod threat_detection;
pub mod secure_storage;
pub mod vulnerability_scanner;
pub mod rate_limiting;

// Re-export main types for convenient access
pub use authentication::{
    AuthenticationManager, AuthMethod, AuthConfig, AuthResult, AuthError,
    TwoFactorAuth, BiometricAuth, SessionManager, SessionToken
};
pub use authorization::{
    AuthorizationManager, Permission, Role, AccessPolicy, ResourceAccess,
    PolicyEngine, RoleBasedAccess, AttributeBasedAccess
};
pub use encryption::{
    EncryptionManager, CipherSuite, KeyManager, EncryptionConfig,
    AsymmetricEncryption, SymmetricEncryption, HashManager, DigitalSignature
};
pub use input_validation::{
    InputValidator, ValidationRule, SanitizationConfig, ValidationError,
    EmailValidator, UrlValidator, FileValidator, SqlInjectionProtection
};
pub use network_security::{
    NetworkSecurityManager, TlsConfig, CertificateManager, FirewallRules,
    DdosProtection, IntrusionDetection, SecureTransport
};
pub use audit_logging::{
    AuditLogger, AuditEvent, SecurityEvent, ComplianceLogger,
    ForensicAnalyzer, LogRetention, EventCorrelation
};
pub use threat_detection::{
    ThreatDetector, ThreatIntelligence, AnomalyDetector, BehaviorAnalyzer,
    MalwareScanner, SecurityAlert, ThreatResponse
};
pub use secure_storage::{
    SecureVault, CredentialManager, SecretStorage, KeyVault,
    EncryptedDatabase, SecureFileSystem, DataClassification
};
pub use vulnerability_scanner::{
    VulnerabilityScanner, SecurityScanner, ComplianceChecker,
    PenetrationTester, SecurityAssessment, VulnerabilityReport
};
pub use rate_limiting::{
    RateLimiter, RateLimitConfig, TokenBucket, SlidingWindow,
    AdaptiveRateLimiting, BruteForceProtection, ThrottlingManager
};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use thiserror::Error;

/// Comprehensive security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Authentication configuration
    pub authentication: AuthConfig,
    /// Encryption settings
    pub encryption: EncryptionConfig,
    /// Network security configuration
    pub network_security: NetworkSecurityConfig,
    /// Input validation rules
    pub input_validation: ValidationConfig,
    /// Rate limiting configuration  
    pub rate_limiting: RateLimitingConfig,
    /// Audit logging settings
    pub audit_logging: AuditConfig,
    /// Threat detection configuration
    pub threat_detection: ThreatDetectionConfig,
    /// Security monitoring intervals
    pub monitoring_intervals: SecurityMonitoringIntervals,
}

/// Network security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkSecurityConfig {
    /// Enable TLS/SSL for all connections
    pub enforce_tls: bool,
    /// Minimum TLS version
    pub min_tls_version: String,
    /// Certificate validation strictness
    pub strict_certificate_validation: bool,
    /// Enable certificate pinning
    pub certificate_pinning: bool,
    /// HSTS max age in seconds
    pub hsts_max_age: u64,
    /// Enable OCSP stapling
    pub ocsp_stapling: bool,
    /// Allowed cipher suites
    pub allowed_ciphers: Vec<String>,
    /// Enable perfect forward secrecy
    pub perfect_forward_secrecy: bool,
}

/// Input validation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationConfig {
    /// Enable input sanitization
    pub enable_sanitization: bool,
    /// Maximum input length
    pub max_input_length: usize,
    /// Enable SQL injection protection
    pub sql_injection_protection: bool,
    /// Enable XSS protection
    pub xss_protection: bool,
    /// Enable path traversal protection
    pub path_traversal_protection: bool,
    /// Custom validation rules
    pub custom_rules: HashMap<String, String>,
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitingConfig {
    /// Enable rate limiting
    pub enabled: bool,
    /// Default rate limit (requests per minute)
    pub default_rate_limit: u32,
    /// Burst allowance
    pub burst_allowance: u32,
    /// Blacklist duration for violations
    pub blacklist_duration: Duration,
    /// Enable adaptive rate limiting
    pub adaptive_limiting: bool,
    /// Brute force protection threshold
    pub brute_force_threshold: u32,
}

/// Audit logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditConfig {
    /// Enable audit logging
    pub enabled: bool,
    /// Log retention period
    pub retention_period: Duration,
    /// Enable real-time monitoring
    pub real_time_monitoring: bool,
    /// Enable compliance reporting
    pub compliance_reporting: bool,
    /// Log encryption enabled
    pub encrypt_logs: bool,
    /// Maximum log file size
    pub max_log_size: usize,
}

/// Threat detection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatDetectionConfig {
    /// Enable real-time threat detection
    pub enabled: bool,
    /// Anomaly detection sensitivity (0.0-1.0)
    pub anomaly_sensitivity: f64,
    /// Enable behavioral analysis
    pub behavioral_analysis: bool,
    /// Enable malware scanning
    pub malware_scanning: bool,
    /// Threat intelligence sources
    pub threat_intel_sources: Vec<String>,
    /// Alert threshold
    pub alert_threshold: ThreatLevel,
}

/// Security monitoring intervals
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityMonitoringIntervals {
    /// Security scan interval
    pub security_scan_seconds: u64,
    /// Vulnerability assessment interval
    pub vulnerability_scan_seconds: u64,
    /// Threat detection interval
    pub threat_detection_seconds: u64,
    /// Audit log analysis interval
    pub audit_analysis_seconds: u64,
    /// Certificate expiry check interval
    pub certificate_check_seconds: u64,
}

/// Security threat levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ThreatLevel {
    Low = 1,
    Medium = 2,
    High = 3,
    Critical = 4,
}

/// Security event types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityEventType {
    AuthenticationFailure,
    AuthorizationViolation,
    SuspiciousActivity,
    MalwareDetected,
    DataBreach,
    VulnerabilityFound,
    ConfigurationChange,
    AccessViolation,
    NetworkIntrusion,
    CertificateExpiry,
}

/// Security incident
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityIncident {
    /// Incident ID
    pub id: String,
    /// Incident type
    pub incident_type: SecurityEventType,
    /// Severity level
    pub severity: ThreatLevel,
    /// Incident timestamp
    pub timestamp: Instant,
    /// Source of incident
    pub source: String,
    /// Affected resource
    pub affected_resource: String,
    /// Incident description
    pub description: String,
    /// Incident details
    pub details: HashMap<String, String>,
    /// Response actions taken
    pub response_actions: Vec<String>,
    /// Incident status
    pub status: IncidentStatus,
}

/// Security incident status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IncidentStatus {
    Open,
    InProgress,
    Resolved,
    False Positive,
}

/// Security metrics aggregation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityMetrics {
    /// Total security events
    pub total_events: u64,
    /// Events by severity
    pub events_by_severity: HashMap<ThreatLevel, u64>,
    /// Authentication success rate
    pub auth_success_rate: f64,
    /// Failed authentication attempts
    pub failed_auth_attempts: u64,
    /// Blocked attacks
    pub blocked_attacks: u64,
    /// Vulnerabilities found
    pub vulnerabilities_found: u64,
    /// Security scan results
    pub scan_results: SecurityScanResults,
    /// Last security assessment
    pub last_assessment: Option<Instant>,
}

/// Security scan results summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityScanResults {
    /// Critical vulnerabilities
    pub critical_vulnerabilities: u64,
    /// High severity vulnerabilities
    pub high_vulnerabilities: u64,
    /// Medium severity vulnerabilities  
    pub medium_vulnerabilities: u64,
    /// Low severity vulnerabilities
    pub low_vulnerabilities: u64,
    /// Security score (0.0-1.0)
    pub security_score: f64,
    /// Compliance score (0.0-1.0)
    pub compliance_score: f64,
    /// Last scan timestamp
    pub last_scan: Option<Instant>,
}

/// Comprehensive security errors
#[derive(Debug, Error)]
pub enum SecurityError {
    #[error("Authentication failed: {0}")]
    AuthenticationError(String),
    
    #[error("Authorization denied: {0}")]
    AuthorizationError(String),
    
    #[error("Encryption error: {0}")]
    EncryptionError(String),
    
    #[error("Input validation failed: {0}")]
    ValidationError(String),
    
    #[error("Network security violation: {0}")]
    NetworkSecurityError(String),
    
    #[error("Rate limit exceeded: {0}")]
    RateLimitError(String),
    
    #[error("Security threat detected: {0}")]
    ThreatDetectionError(String),
    
    #[error("Audit logging error: {0}")]
    AuditError(String),
    
    #[error("Vulnerability found: {0}")]
    VulnerabilityError(String),
    
    #[error("Configuration error: {0}")]
    ConfigurationError(String),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

pub type SecurityResult<T> = Result<T, SecurityError>;

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            authentication: AuthConfig::default(),
            encryption: EncryptionConfig::default(),
            network_security: NetworkSecurityConfig::default(),
            input_validation: ValidationConfig::default(),
            rate_limiting: RateLimitingConfig::default(),
            audit_logging: AuditConfig::default(),
            threat_detection: ThreatDetectionConfig::default(),
            monitoring_intervals: SecurityMonitoringIntervals::default(),
        }
    }
}

impl Default for NetworkSecurityConfig {
    fn default() -> Self {
        Self {
            enforce_tls: true,
            min_tls_version: "TLSv1.3".to_string(),
            strict_certificate_validation: true,
            certificate_pinning: false,
            hsts_max_age: 31536000, // 1 year
            ocsp_stapling: true,
            allowed_ciphers: vec![
                "TLS_AES_256_GCM_SHA384".to_string(),
                "TLS_CHACHA20_POLY1305_SHA256".to_string(),
                "TLS_AES_128_GCM_SHA256".to_string(),
            ],
            perfect_forward_secrecy: true,
        }
    }
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            enable_sanitization: true,
            max_input_length: 10000,
            sql_injection_protection: true,
            xss_protection: true,
            path_traversal_protection: true,
            custom_rules: HashMap::new(),
        }
    }
}

impl Default for RateLimitingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_rate_limit: 60, // 60 requests per minute
            burst_allowance: 10,
            blacklist_duration: Duration::from_secs(300), // 5 minutes
            adaptive_limiting: true,
            brute_force_threshold: 5,
        }
    }
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            retention_period: Duration::from_secs(90 * 24 * 3600), // 90 days
            real_time_monitoring: true,
            compliance_reporting: true,
            encrypt_logs: true,
            max_log_size: 100 * 1024 * 1024, // 100MB
        }
    }
}

impl Default for ThreatDetectionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            anomaly_sensitivity: 0.7,
            behavioral_analysis: true,
            malware_scanning: true,
            threat_intel_sources: vec![
                "local_database".to_string(),
                "security_feeds".to_string(),
            ],
            alert_threshold: ThreatLevel::Medium,
        }
    }
}

impl Default for SecurityMonitoringIntervals {
    fn default() -> Self {
        Self {
            security_scan_seconds: 3600, // 1 hour
            vulnerability_scan_seconds: 86400, // 24 hours
            threat_detection_seconds: 300, // 5 minutes
            audit_analysis_seconds: 1800, // 30 minutes
            certificate_check_seconds: 43200, // 12 hours
        }
    }
}

impl std::fmt::Display for ThreatLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ThreatLevel::Low => write!(f, "Low"),
            ThreatLevel::Medium => write!(f, "Medium"),
            ThreatLevel::High => write!(f, "High"),
            ThreatLevel::Critical => write!(f, "Critical"),
        }
    }
}