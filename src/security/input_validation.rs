//! Comprehensive input validation and sanitization system

use crate::security::{SecurityResult, SecurityError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use regex::Regex;
use url::Url;
use once_cell::sync::Lazy;

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
    /// Enable command injection protection
    pub command_injection_protection: bool,
    /// Enable LDAP injection protection
    pub ldap_injection_protection: bool,
    /// Custom validation rules
    pub custom_rules: HashMap<String, String>,
    /// Whitelist configuration
    pub whitelist: WhitelistConfig,
    /// Strict mode (reject on any suspicious input)
    pub strict_mode: bool,
}

/// Whitelist configuration for allowed inputs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhitelistConfig {
    /// Allowed characters for usernames
    pub username_chars: String,
    /// Allowed characters for passwords
    pub password_chars: String,
    /// Allowed file extensions
    pub file_extensions: Vec<String>,
    /// Allowed MIME types
    pub mime_types: Vec<String>,
    /// Allowed domains
    pub domains: Vec<String>,
    /// Allowed IP address ranges
    pub ip_ranges: Vec<String>,
}

/// Sanitization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SanitizationConfig {
    /// Remove HTML tags
    pub strip_html: bool,
    /// Escape HTML entities
    pub escape_html: bool,
    /// Remove JavaScript
    pub strip_javascript: bool,
    /// Remove SQL keywords
    pub strip_sql_keywords: bool,
    /// Normalize Unicode
    pub normalize_unicode: bool,
    /// Trim whitespace
    pub trim_whitespace: bool,
    /// Convert to lowercase
    pub force_lowercase: bool,
    /// Remove null bytes
    pub remove_null_bytes: bool,
}

/// Validation rule definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRule {
    /// Rule name
    pub name: String,
    /// Rule description
    pub description: String,
    /// Rule type
    pub rule_type: RuleType,
    /// Rule pattern (regex or other)
    pub pattern: String,
    /// Error message for rule violation
    pub error_message: String,
    /// Rule severity
    pub severity: ValidationSeverity,
    /// Whether rule is enabled
    pub enabled: bool,
}

/// Types of validation rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuleType {
    /// Regular expression pattern
    Regex,
    /// Length constraint
    Length,
    /// Character whitelist
    Whitelist,
    /// Character blacklist
    Blacklist,
    /// Custom function
    Custom,
    /// Format validation (email, URL, etc.)
    Format,
}

/// Validation severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ValidationSeverity {
    Info = 1,
    Warning = 2,
    Error = 3,
    Critical = 4,
}

/// Validation result
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether input passed validation
    pub is_valid: bool,
    /// Sanitized input (if sanitization was performed)
    pub sanitized_input: Option<String>,
    /// Validation errors found
    pub errors: Vec<ValidationError>,
    /// Warnings generated
    pub warnings: Vec<ValidationWarning>,
    /// Applied rules
    pub applied_rules: Vec<String>,
    /// Risk score (0.0-1.0)
    pub risk_score: f64,
}

/// Validation error details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    /// Rule that triggered the error
    pub rule_name: String,
    /// Error message
    pub message: String,
    /// Error severity
    pub severity: ValidationSeverity,
    /// Position in input where error occurred
    pub position: Option<usize>,
    /// Offending substring
    pub offending_text: Option<String>,
    /// Suggested fix
    pub suggested_fix: Option<String>,
}

/// Validation warning details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationWarning {
    /// Rule that triggered the warning
    pub rule_name: String,
    /// Warning message
    pub message: String,
    /// Position in input where warning occurred
    pub position: Option<usize>,
    /// Suspicious pattern found
    pub pattern: Option<String>,
}

/// Comprehensive input validator
pub struct InputValidator {
    /// Configuration
    config: ValidationConfig,
    /// Validation rules
    rules: HashMap<String, ValidationRule>,
    /// Sanitization configuration
    sanitization: SanitizationConfig,
    /// Pre-compiled regex patterns
    regex_cache: HashMap<String, Regex>,
}

impl InputValidator {
    /// Create a new input validator
    pub fn new(config: ValidationConfig) -> SecurityResult<Self> {
        let mut validator = Self {
            config: config.clone(),
            rules: HashMap::new(),
            sanitization: SanitizationConfig::default(),
            regex_cache: HashMap::new(),
        };

        // Initialize default validation rules
        validator.initialize_default_rules()?;
        
        Ok(validator)
    }

    /// Validate input against all applicable rules
    pub fn validate(&mut self, input: &str, context: &str) -> SecurityResult<ValidationResult> {
        let mut result = ValidationResult {
            is_valid: true,
            sanitized_input: None,
            errors: Vec::new(),
            warnings: Vec::new(),
            applied_rules: Vec::new(),
            risk_score: 0.0,
        };

        // Check input length first
        if input.len() > self.config.max_input_length {
            result.is_valid = false;
            result.errors.push(ValidationError {
                rule_name: "max_length".to_string(),
                message: format!("Input exceeds maximum length of {} characters", self.config.max_input_length),
                severity: ValidationSeverity::Error,
                position: Some(self.config.max_input_length),
                offending_text: Some(input[self.config.max_input_length..].to_string()),
                suggested_fix: Some(format!("Truncate input to {} characters", self.config.max_input_length)),
            });
        }

        // Apply context-specific validation
        match context {
            "email" => self.validate_email(input, &mut result)?,
            "url" => self.validate_url(input, &mut result)?,
            "username" => self.validate_username(input, &mut result)?,
            "password" => self.validate_password(input, &mut result)?,
            "filename" => self.validate_filename(input, &mut result)?,
            "sql_query" => self.validate_sql_query(input, &mut result)?,
            _ => self.validate_general(input, &mut result)?,
        }

        // Apply sanitization if requested
        if self.config.enable_sanitization {
            result.sanitized_input = Some(self.sanitize_input(input)?);
        }

        // Calculate risk score
        result.risk_score = self.calculate_risk_score(&result);

        // In strict mode, any error makes input invalid
        if self.config.strict_mode && !result.errors.is_empty() {
            result.is_valid = false;
        }

        Ok(result)
    }

    /// Sanitize input according to configuration
    pub fn sanitize_input(&self, input: &str) -> SecurityResult<String> {
        let mut sanitized = input.to_string();

        // Remove null bytes
        if self.sanitization.remove_null_bytes {
            sanitized = sanitized.replace('\0', "");
        }

        // Normalize Unicode
        if self.sanitization.normalize_unicode {
            sanitized = self.normalize_unicode(&sanitized);
        }

        // Strip HTML if configured
        if self.sanitization.strip_html {
            sanitized = self.strip_html(&sanitized);
        }

        // Escape HTML entities if configured
        if self.sanitization.escape_html {
            sanitized = self.escape_html(&sanitized);
        }

        // Strip JavaScript
        if self.sanitization.strip_javascript {
            sanitized = self.strip_javascript(&sanitized);
        }

        // Strip SQL keywords
        if self.sanitization.strip_sql_keywords {
            sanitized = self.strip_sql_keywords(&sanitized);
        }

        // Trim whitespace
        if self.sanitization.trim_whitespace {
            sanitized = sanitized.trim().to_string();
        }

        // Force lowercase
        if self.sanitization.force_lowercase {
            sanitized = sanitized.to_lowercase();
        }

        Ok(sanitized)
    }

    /// Validate email addresses
    fn validate_email(&mut self, email: &str, result: &mut ValidationResult) -> SecurityResult<()> {
        result.applied_rules.push("email_format".to_string());

        // Basic email format validation
        let email_regex = self.get_or_compile_regex("email", EMAIL_REGEX)?;
        
        if !email_regex.is_match(email) {
            result.is_valid = false;
            result.errors.push(ValidationError {
                rule_name: "email_format".to_string(),
                message: "Invalid email format".to_string(),
                severity: ValidationSeverity::Error,
                position: None,
                offending_text: Some(email.to_string()),
                suggested_fix: Some("Use format: user@domain.com".to_string()),
            });
            return Ok(());
        }

        // Check for common email injection patterns
        let dangerous_patterns = [
            r"[<>\"'();]",
            r"\b(bcc|cc):",
            r"content-type:",
            r"mime-version:",
        ];

        for (i, pattern) in dangerous_patterns.iter().enumerate() {
            let regex = self.get_or_compile_regex(&format!("email_injection_{}", i), pattern)?;
            if regex.is_match(email) {
                result.warnings.push(ValidationWarning {
                    rule_name: "email_injection".to_string(),
                    message: "Potentially dangerous email pattern detected".to_string(),
                    position: None,
                    pattern: Some(pattern.to_string()),
                });
            }
        }

        Ok(())
    }

    /// Validate URLs
    fn validate_url(&mut self, url_str: &str, result: &mut ValidationResult) -> SecurityResult<()> {
        result.applied_rules.push("url_format".to_string());

        // Parse URL
        match Url::parse(url_str) {
            Ok(url) => {
                // Check scheme
                if !["http", "https", "ftp", "ftps"].contains(&url.scheme()) {
                    result.warnings.push(ValidationWarning {
                        rule_name: "url_scheme".to_string(),
                        message: format!("Unusual URL scheme: {}", url.scheme()),
                        position: None,
                        pattern: Some(url.scheme().to_string()),
                    });
                }

                // Check for suspicious patterns in URL
                if url_str.contains("javascript:") || url_str.contains("data:") || url_str.contains("vbscript:") {
                    result.is_valid = false;
                    result.errors.push(ValidationError {
                        rule_name: "url_dangerous_scheme".to_string(),
                        message: "Dangerous URL scheme detected".to_string(),
                        severity: ValidationSeverity::Critical,
                        position: None,
                        offending_text: Some(url_str.to_string()),
                        suggested_fix: Some("Use http:// or https:// URLs only".to_string()),
                    });
                }

                // Check domain whitelist if configured
                if !self.config.whitelist.domains.is_empty() {
                    if let Some(host) = url.host_str() {
                        if !self.config.whitelist.domains.iter().any(|domain| host.ends_with(domain)) {
                            result.warnings.push(ValidationWarning {
                                rule_name: "url_domain_whitelist".to_string(),
                                message: format!("Domain '{}' not in whitelist", host),
                                position: None,
                                pattern: Some(host.to_string()),
                            });
                        }
                    }
                }
            }
            Err(_) => {
                result.is_valid = false;
                result.errors.push(ValidationError {
                    rule_name: "url_format".to_string(),
                    message: "Invalid URL format".to_string(),
                    severity: ValidationSeverity::Error,
                    position: None,
                    offending_text: Some(url_str.to_string()),
                    suggested_fix: Some("Use valid URL format: https://example.com".to_string()),
                });
            }
        }

        Ok(())
    }

    /// Validate usernames
    fn validate_username(&mut self, username: &str, result: &mut ValidationResult) -> SecurityResult<()> {
        result.applied_rules.push("username_format".to_string());

        // Check length
        if username.len() < 3 {
            result.is_valid = false;
            result.errors.push(ValidationError {
                rule_name: "username_length".to_string(),
                message: "Username must be at least 3 characters long".to_string(),
                severity: ValidationSeverity::Error,
                position: None,
                offending_text: Some(username.to_string()),
                suggested_fix: Some("Use a username with at least 3 characters".to_string()),
            });
        }

        if username.len() > 32 {
            result.is_valid = false;
            result.errors.push(ValidationError {
                rule_name: "username_length".to_string(),
                message: "Username cannot exceed 32 characters".to_string(),
                severity: ValidationSeverity::Error,
                position: Some(32),
                offending_text: Some(username[32..].to_string()),
                suggested_fix: Some("Use a username with 32 characters or fewer".to_string()),
            });
        }

        // Check allowed characters
        let allowed_chars = &self.config.whitelist.username_chars;
        if !allowed_chars.is_empty() {
            for (i, ch) in username.char_indices() {
                if !allowed_chars.contains(ch) {
                    result.is_valid = false;
                    result.errors.push(ValidationError {
                        rule_name: "username_characters".to_string(),
                        message: format!("Invalid character '{}' in username", ch),
                        severity: ValidationSeverity::Error,
                        position: Some(i),
                        offending_text: Some(ch.to_string()),
                        suggested_fix: Some(format!("Use only these characters: {}", allowed_chars)),
                    });
                }
            }
        }

        // Check for reserved usernames
        let reserved_usernames = [
            "admin", "root", "administrator", "system", "guest", "anonymous",
            "null", "undefined", "test", "demo", "api", "www"
        ];
        
        if reserved_usernames.contains(&username.to_lowercase().as_str()) {
            result.warnings.push(ValidationWarning {
                rule_name: "username_reserved".to_string(),
                message: "Username is reserved or commonly used".to_string(),
                position: None,
                pattern: Some(username.to_string()),
            });
        }

        Ok(())
    }

    /// Validate passwords
    fn validate_password(&mut self, password: &str, result: &mut ValidationResult) -> SecurityResult<()> {
        result.applied_rules.push("password_strength".to_string());

        let mut strength_score = 0;
        let mut requirements_met = Vec::new();
        let mut requirements_failed = Vec::new();

        // Length check
        if password.len() >= 12 {
            strength_score += 2;
            requirements_met.push("minimum length");
        } else {
            requirements_failed.push("at least 12 characters");
        }

        // Uppercase letters
        if password.chars().any(|c| c.is_uppercase()) {
            strength_score += 1;
            requirements_met.push("uppercase letters");
        } else {
            requirements_failed.push("uppercase letters");
        }

        // Lowercase letters
        if password.chars().any(|c| c.is_lowercase()) {
            strength_score += 1;
            requirements_met.push("lowercase letters");
        } else {
            requirements_failed.push("lowercase letters");
        }

        // Numbers
        if password.chars().any(|c| c.is_numeric()) {
            strength_score += 1;
            requirements_met.push("numbers");
        } else {
            requirements_failed.push("numbers");
        }

        // Special characters
        if password.chars().any(|c| "!@#$%^&*()_+-=[]{}|;:,.<>?".contains(c)) {
            strength_score += 1;
            requirements_met.push("special characters");
        } else {
            requirements_failed.push("special characters");
        }

        // Check for common weak passwords
        if self.is_common_password(password) {
            result.is_valid = false;
            result.errors.push(ValidationError {
                rule_name: "password_common".to_string(),
                message: "Password is too common and easily guessable".to_string(),
                severity: ValidationSeverity::Critical,
                position: None,
                offending_text: None, // Don't include password in logs
                suggested_fix: Some("Use a unique password with mixed characters".to_string()),
            });
        }

        // Overall strength assessment
        if strength_score < 3 {
            result.is_valid = false;
            result.errors.push(ValidationError {
                rule_name: "password_strength".to_string(),
                message: format!("Password is too weak. Missing: {}", requirements_failed.join(", ")),
                severity: ValidationSeverity::Error,
                position: None,
                offending_text: None,
                suggested_fix: Some("Include uppercase, lowercase, numbers, and special characters".to_string()),
            });
        } else if strength_score < 5 {
            result.warnings.push(ValidationWarning {
                rule_name: "password_strength".to_string(),
                message: format!("Password could be stronger. Consider adding: {}", requirements_failed.join(", ")),
                position: None,
                pattern: None,
            });
        }

        Ok(())
    }

    /// Validate filenames
    fn validate_filename(&mut self, filename: &str, result: &mut ValidationResult) -> SecurityResult<()> {
        result.applied_rules.push("filename_security".to_string());

        // Check for path traversal attempts
        if self.config.path_traversal_protection {
            let dangerous_patterns = ["../", "..\\", "/.", "\\..", "%2e%2e", "%252e%252e"];
            for pattern in &dangerous_patterns {
                if filename.contains(pattern) {
                    result.is_valid = false;
                    result.errors.push(ValidationError {
                        rule_name: "path_traversal".to_string(),
                        message: "Path traversal attempt detected in filename".to_string(),
                        severity: ValidationSeverity::Critical,
                        position: filename.find(pattern),
                        offending_text: Some(pattern.to_string()),
                        suggested_fix: Some("Use simple filename without path components".to_string()),
                    });
                }
            }
        }

        // Check for dangerous characters
        let dangerous_chars = ['<', '>', ':', '"', '|', '?', '*', '\0'];
        for (i, ch) in filename.char_indices() {
            if dangerous_chars.contains(&ch) {
                result.warnings.push(ValidationWarning {
                    rule_name: "filename_chars".to_string(),
                    message: format!("Potentially dangerous character '{}' in filename", ch),
                    position: Some(i),
                    pattern: Some(ch.to_string()),
                });
            }
        }

        // Check file extension whitelist
        if !self.config.whitelist.file_extensions.is_empty() {
            if let Some(extension) = filename.split('.').last() {
                if !self.config.whitelist.file_extensions.contains(&extension.to_lowercase()) {
                    result.warnings.push(ValidationWarning {
                        rule_name: "file_extension".to_string(),
                        message: format!("File extension '{}' not in whitelist", extension),
                        position: None,
                        pattern: Some(extension.to_string()),
                    });
                }
            }
        }

        // Check for executable extensions
        let executable_extensions = ["exe", "bat", "cmd", "com", "scr", "pif", "js", "vbs", "ps1", "sh"];
        if let Some(extension) = filename.split('.').last() {
            if executable_extensions.contains(&extension.to_lowercase().as_str()) {
                result.warnings.push(ValidationWarning {
                    rule_name: "executable_file".to_string(),
                    message: "File appears to be executable".to_string(),
                    position: None,
                    pattern: Some(extension.to_string()),
                });
            }
        }

        Ok(())
    }

    /// Validate SQL queries
    fn validate_sql_query(&mut self, query: &str, result: &mut ValidationResult) -> SecurityResult<()> {
        if !self.config.sql_injection_protection {
            return Ok(());
        }

        result.applied_rules.push("sql_injection_protection".to_string());

        // Check for SQL injection patterns
        let injection_patterns = [
            r"(?i)\b(union\s+select|or\s+1\s*=\s*1|and\s+1\s*=\s*1)",
            r"(?i)\b(drop\s+table|delete\s+from|insert\s+into)\b",
            r"(?i)\b(exec\s*\(|execute\s*\(|sp_|xp_)",
            r"(?i)(\-\-|\#|\/\*)",
            r"(?i)\b(script|javascript|vbscript)\b",
            r"['\";].*(\bor\b|\band\b).*['\";]",
        ];

        for (i, pattern) in injection_patterns.iter().enumerate() {
            let regex = self.get_or_compile_regex(&format!("sql_injection_{}", i), pattern)?;
            if let Some(mat) = regex.find(query) {
                result.is_valid = false;
                result.errors.push(ValidationError {
                    rule_name: "sql_injection".to_string(),
                    message: "Potential SQL injection attempt detected".to_string(),
                    severity: ValidationSeverity::Critical,
                    position: Some(mat.start()),
                    offending_text: Some(mat.as_str().to_string()),
                    suggested_fix: Some("Use parameterized queries".to_string()),
                });
            }
        }

        Ok(())
    }

    /// General validation for any input
    fn validate_general(&mut self, input: &str, result: &mut ValidationResult) -> SecurityResult<()> {
        result.applied_rules.push("general_security".to_string());

        // XSS protection
        if self.config.xss_protection {
            self.check_xss_patterns(input, result)?;
        }

        // Command injection protection
        if self.config.command_injection_protection {
            self.check_command_injection(input, result)?;
        }

        // LDAP injection protection
        if self.config.ldap_injection_protection {
            self.check_ldap_injection(input, result)?;
        }

        // Check for suspicious patterns
        self.check_suspicious_patterns(input, result)?;

        Ok(())
    }

    /// Check for XSS patterns
    fn check_xss_patterns(&mut self, input: &str, result: &mut ValidationResult) -> SecurityResult<()> {
        let xss_patterns = [
            r"(?i)<script[^>]*>.*?</script>",
            r"(?i)javascript:",
            r"(?i)vbscript:",
            r"(?i)onload\s*=",
            r"(?i)onerror\s*=",
            r"(?i)onclick\s*=",
            r"(?i)<iframe[^>]*>",
            r"(?i)<object[^>]*>",
            r"(?i)<embed[^>]*>",
            r"(?i)expression\s*\(",
        ];

        for (i, pattern) in xss_patterns.iter().enumerate() {
            let regex = self.get_or_compile_regex(&format!("xss_{}", i), pattern)?;
            if let Some(mat) = regex.find(input) {
                result.is_valid = false;
                result.errors.push(ValidationError {
                    rule_name: "xss_protection".to_string(),
                    message: "Potential XSS attempt detected".to_string(),
                    severity: ValidationSeverity::Critical,
                    position: Some(mat.start()),
                    offending_text: Some(mat.as_str().to_string()),
                    suggested_fix: Some("Remove or escape HTML/JavaScript content".to_string()),
                });
            }
        }

        Ok(())
    }

    /// Check for command injection patterns
    fn check_command_injection(&mut self, input: &str, result: &mut ValidationResult) -> SecurityResult<()> {
        let command_patterns = [
            r"[;&|`$]",
            r"(?i)\b(cmd|bash|sh|powershell)\b",
            r"(?i)\b(rm\s+\-rf|del\s+/s|format\s+c:)\b",
            r"(?i)\b(wget|curl|nc|netcat)\b",
        ];

        for (i, pattern) in command_patterns.iter().enumerate() {
            let regex = self.get_or_compile_regex(&format!("cmd_injection_{}", i), pattern)?;
            if let Some(mat) = regex.find(input) {
                result.warnings.push(ValidationWarning {
                    rule_name: "command_injection".to_string(),
                    message: "Potential command injection pattern detected".to_string(),
                    position: Some(mat.start()),
                    pattern: Some(mat.as_str().to_string()),
                });
            }
        }

        Ok(())
    }

    /// Check for LDAP injection patterns
    fn check_ldap_injection(&mut self, input: &str, result: &mut ValidationResult) -> SecurityResult<()> {
        let ldap_patterns = [
            r"[()=*|&]",
            r"\x00",
            r"\\[0-9a-fA-F]{2}",
        ];

        for (i, pattern) in ldap_patterns.iter().enumerate() {
            let regex = self.get_or_compile_regex(&format!("ldap_injection_{}", i), pattern)?;
            if regex.is_match(input) {
                result.warnings.push(ValidationWarning {
                    rule_name: "ldap_injection".to_string(),
                    message: "Potential LDAP injection pattern detected".to_string(),
                    position: None,
                    pattern: Some(pattern.to_string()),
                });
            }
        }

        Ok(())
    }

    /// Check for generally suspicious patterns
    fn check_suspicious_patterns(&mut self, input: &str, result: &mut ValidationResult) -> SecurityResult<()> {
        // Check for excessive repeating characters
        let repeat_regex = self.get_or_compile_regex("repeat_chars", r"(.)\1{20,}")?;
        if repeat_regex.is_match(input) {
            result.warnings.push(ValidationWarning {
                rule_name: "suspicious_pattern".to_string(),
                message: "Excessive repeating characters detected".to_string(),
                position: None,
                pattern: Some("repeating_chars".to_string()),
            });
        }

        // Check for binary data
        let non_printable_count = input.chars().filter(|c| c.is_control() && *c != '\n' && *c != '\t').count();
        if non_printable_count > input.len() / 10 {
            result.warnings.push(ValidationWarning {
                rule_name: "suspicious_pattern".to_string(),
                message: "High concentration of non-printable characters detected".to_string(),
                position: None,
                pattern: Some("binary_data".to_string()),
            });
        }

        Ok(())
    }

    /// Initialize default validation rules
    fn initialize_default_rules(&mut self) -> SecurityResult<()> {
        let default_rules = vec![
            ValidationRule {
                name: "no_nulls".to_string(),
                description: "Reject null bytes".to_string(),
                rule_type: RuleType::Regex,
                pattern: r"\x00".to_string(),
                error_message: "Null bytes are not allowed".to_string(),
                severity: ValidationSeverity::Critical,
                enabled: true,
            },
            ValidationRule {
                name: "length_limit".to_string(),
                description: "Enforce maximum length".to_string(),
                rule_type: RuleType::Length,
                pattern: self.config.max_input_length.to_string(),
                error_message: "Input exceeds maximum allowed length".to_string(),
                severity: ValidationSeverity::Error,
                enabled: true,
            },
        ];

        for rule in default_rules {
            self.rules.insert(rule.name.clone(), rule);
        }

        Ok(())
    }

    /// Get or compile regex pattern
    fn get_or_compile_regex(&mut self, name: &str, pattern: &str) -> SecurityResult<&Regex> {
        if !self.regex_cache.contains_key(name) {
            let regex = Regex::new(pattern)
                .map_err(|e| SecurityError::ValidationError(format!("Invalid regex '{}': {}", pattern, e)))?;
            self.regex_cache.insert(name.to_string(), regex);
        }
        
        Ok(self.regex_cache.get(name).unwrap())
    }

    /// Helper methods for sanitization
    fn normalize_unicode(&self, input: &str) -> String {
        // Basic Unicode normalization
        input.chars().filter(|c| !c.is_control() || *c == '\n' || *c == '\t').collect()
    }

    fn strip_html(&self, input: &str) -> String {
        // Basic HTML tag removal
        let tag_regex = Regex::new(r"<[^>]+>").unwrap();
        tag_regex.replace_all(input, "").to_string()
    }

    fn escape_html(&self, input: &str) -> String {
        input
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&#x27;")
    }

    fn strip_javascript(&self, input: &str) -> String {
        // Remove common JavaScript patterns
        let js_regex = Regex::new(r"(?i)(javascript:|vbscript:|on\w+\s*=)").unwrap();
        js_regex.replace_all(input, "").to_string()
    }

    fn strip_sql_keywords(&self, input: &str) -> String {
        // Remove dangerous SQL keywords
        let sql_regex = Regex::new(r"(?i)\b(drop|delete|insert|update|union|select|exec|execute)\b").unwrap();
        sql_regex.replace_all(input, "").to_string()
    }

    fn is_common_password(&self, password: &str) -> bool {
        // Check against common password list
        let common_passwords = [
            "password", "123456", "password123", "admin", "qwerty",
            "letmein", "welcome", "monkey", "dragon", "master"
        ];
        
        common_passwords.contains(&password.to_lowercase().as_str())
    }

    fn calculate_risk_score(&self, result: &ValidationResult) -> f64 {
        let mut risk_score = 0.0;
        
        for error in &result.errors {
            risk_score += match error.severity {
                ValidationSeverity::Critical => 0.4,
                ValidationSeverity::Error => 0.2,
                ValidationSeverity::Warning => 0.1,
                ValidationSeverity::Info => 0.05,
            };
        }

        for warning in &result.warnings {
            risk_score += 0.05;
        }

        risk_score.min(1.0)
    }
}

// Regular expressions for common validations
static EMAIL_REGEX: &str = r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$";
static URL_REGEX: &str = r"^https?://[^\s/$.?#].[^\s]*$";

// Specialized validators
pub struct EmailValidator;
pub struct UrlValidator;
pub struct FileValidator;
pub struct SqlInjectionProtection;

impl EmailValidator {
    pub fn validate(email: &str) -> ValidationResult {
        let regex = Regex::new(EMAIL_REGEX).unwrap();
        let is_valid = regex.is_match(email);
        
        ValidationResult {
            is_valid,
            sanitized_input: None,
            errors: if !is_valid { vec![
                ValidationError {
                    rule_name: "email_format".to_string(),
                    message: "Invalid email format".to_string(),
                    severity: ValidationSeverity::Error,
                    position: None,
                    offending_text: Some(email.to_string()),
                    suggested_fix: Some("Use format: user@domain.com".to_string()),
                }
            ] } else { Vec::new() },
            warnings: Vec::new(),
            applied_rules: vec!["email_format".to_string()],
            risk_score: if is_valid { 0.0 } else { 0.3 },
        }
    }
}

impl UrlValidator {
    pub fn validate(url: &str) -> ValidationResult {
        match Url::parse(url) {
            Ok(parsed_url) => {
                let mut warnings = Vec::new();
                
                // Check for dangerous schemes
                if ["javascript", "data", "vbscript"].contains(&parsed_url.scheme()) {
                    return ValidationResult {
                        is_valid: false,
                        sanitized_input: None,
                        errors: vec![ValidationError {
                            rule_name: "dangerous_scheme".to_string(),
                            message: "Dangerous URL scheme detected".to_string(),
                            severity: ValidationSeverity::Critical,
                            position: None,
                            offending_text: Some(parsed_url.scheme().to_string()),
                            suggested_fix: Some("Use http or https URLs only".to_string()),
                        }],
                        warnings,
                        applied_rules: vec!["url_validation".to_string()],
                        risk_score: 0.8,
                    };
                }
                
                // Warning for non-standard schemes
                if !["http", "https", "ftp", "ftps"].contains(&parsed_url.scheme()) {
                    warnings.push(ValidationWarning {
                        rule_name: "unusual_scheme".to_string(),
                        message: format!("Unusual URL scheme: {}", parsed_url.scheme()),
                        position: None,
                        pattern: Some(parsed_url.scheme().to_string()),
                    });
                }

                ValidationResult {
                    is_valid: true,
                    sanitized_input: None,
                    errors: Vec::new(),
                    warnings,
                    applied_rules: vec!["url_validation".to_string()],
                    risk_score: if warnings.is_empty() { 0.0 } else { 0.1 },
                }
            }
            Err(_) => ValidationResult {
                is_valid: false,
                sanitized_input: None,
                errors: vec![ValidationError {
                    rule_name: "url_format".to_string(),
                    message: "Invalid URL format".to_string(),
                    severity: ValidationSeverity::Error,
                    position: None,
                    offending_text: Some(url.to_string()),
                    suggested_fix: Some("Use valid URL format".to_string()),
                }],
                warnings: Vec::new(),
                applied_rules: vec!["url_validation".to_string()],
                risk_score: 0.3,
            }
        }
    }
}

impl FileValidator {
    pub fn validate_filename(filename: &str) -> ValidationResult {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Check for path traversal
        if filename.contains("../") || filename.contains("..\\") {
            errors.push(ValidationError {
                rule_name: "path_traversal".to_string(),
                message: "Path traversal detected in filename".to_string(),
                severity: ValidationSeverity::Critical,
                position: None,
                offending_text: Some(filename.to_string()),
                suggested_fix: Some("Use simple filename without path components".to_string()),
            });
        }

        // Check for dangerous extensions
        if let Some(extension) = filename.split('.').last() {
            let dangerous_extensions = ["exe", "bat", "cmd", "scr", "js", "vbs"];
            if dangerous_extensions.contains(&extension.to_lowercase().as_str()) {
                warnings.push(ValidationWarning {
                    rule_name: "dangerous_extension".to_string(),
                    message: "File has potentially dangerous extension".to_string(),
                    position: None,
                    pattern: Some(extension.to_string()),
                });
            }
        }

        ValidationResult {
            is_valid: errors.is_empty(),
            sanitized_input: None,
            errors,
            warnings,
            applied_rules: vec!["filename_validation".to_string()],
            risk_score: if errors.is_empty() { 0.0 } else { 0.5 },
        }
    }
}

impl SqlInjectionProtection {
    pub fn check_query(query: &str) -> ValidationResult {
        let mut errors = Vec::new();
        
        // Common SQL injection patterns
        let injection_patterns = [
            (r"(?i)\bunion\s+select\b", "UNION SELECT injection"),
            (r"(?i)\bor\s+1\s*=\s*1\b", "OR 1=1 injection"),
            (r"(?i)\bdrop\s+table\b", "DROP TABLE attempt"),
            (r"(?i)\bexec\s*\(", "EXEC execution attempt"),
            (r"--", "SQL comment injection"),
        ];

        for (pattern, description) in &injection_patterns {
            if let Ok(regex) = Regex::new(pattern) {
                if let Some(mat) = regex.find(query) {
                    errors.push(ValidationError {
                        rule_name: "sql_injection".to_string(),
                        message: format!("SQL injection detected: {}", description),
                        severity: ValidationSeverity::Critical,
                        position: Some(mat.start()),
                        offending_text: Some(mat.as_str().to_string()),
                        suggested_fix: Some("Use parameterized queries".to_string()),
                    });
                }
            }
        }

        ValidationResult {
            is_valid: errors.is_empty(),
            sanitized_input: None,
            errors,
            warnings: Vec::new(),
            applied_rules: vec!["sql_injection_check".to_string()],
            risk_score: if errors.is_empty() { 0.0 } else { 0.9 },
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
            command_injection_protection: true,
            ldap_injection_protection: true,
            custom_rules: HashMap::new(),
            whitelist: WhitelistConfig::default(),
            strict_mode: false,
        }
    }
}

impl Default for WhitelistConfig {
    fn default() -> Self {
        Self {
            username_chars: "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_-.".to_string(),
            password_chars: String::new(), // No restriction on password characters
            file_extensions: vec![
                "txt".to_string(), "pdf".to_string(), "doc".to_string(), "docx".to_string(),
                "jpg".to_string(), "jpeg".to_string(), "png".to_string(), "gif".to_string(),
            ],
            mime_types: vec![
                "text/plain".to_string(),
                "application/pdf".to_string(),
                "image/jpeg".to_string(),
                "image/png".to_string(),
            ],
            domains: Vec::new(),
            ip_ranges: Vec::new(),
        }
    }
}

impl Default for SanitizationConfig {
    fn default() -> Self {
        Self {
            strip_html: false,
            escape_html: true,
            strip_javascript: true,
            strip_sql_keywords: false,
            normalize_unicode: true,
            trim_whitespace: true,
            force_lowercase: false,
            remove_null_bytes: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email_validation() {
        let result = EmailValidator::validate("user@example.com");
        assert!(result.is_valid);
        assert_eq!(result.risk_score, 0.0);

        let result = EmailValidator::validate("invalid-email");
        assert!(!result.is_valid);
        assert_eq!(result.errors.len(), 1);
        assert!(result.risk_score > 0.0);
    }

    #[test]
    fn test_url_validation() {
        let result = UrlValidator::validate("https://example.com");
        assert!(result.is_valid);

        let result = UrlValidator::validate("javascript:alert('xss')");
        assert!(!result.is_valid);
        assert_eq!(result.errors[0].severity, ValidationSeverity::Critical);
    }

    #[test]
    fn test_filename_validation() {
        let result = FileValidator::validate_filename("document.pdf");
        assert!(result.is_valid);

        let result = FileValidator::validate_filename("../../../etc/passwd");
        assert!(!result.is_valid);
        assert_eq!(result.errors[0].rule_name, "path_traversal");

        let result = FileValidator::validate_filename("malware.exe");
        assert!(result.is_valid); // Valid filename, but warning for extension
        assert_eq!(result.warnings.len(), 1);
    }

    #[test]
    fn test_sql_injection_detection() {
        let result = SqlInjectionProtection::check_query("SELECT * FROM users WHERE id = 1");
        assert!(result.is_valid);

        let result = SqlInjectionProtection::check_query("SELECT * FROM users WHERE id = 1 OR 1=1");
        assert!(!result.is_valid);
        assert_eq!(result.errors[0].severity, ValidationSeverity::Critical);

        let result = SqlInjectionProtection::check_query("SELECT * FROM users; DROP TABLE users;--");
        assert!(!result.is_valid);
        assert!(result.errors.len() > 1); // Multiple injection patterns detected
    }

    #[tokio::test]
    async fn test_input_validator() {
        let config = ValidationConfig::default();
        let mut validator = InputValidator::new(config).unwrap();

        // Test normal email
        let result = validator.validate("user@example.com", "email").unwrap();
        assert!(result.is_valid);

        // Test XSS attempt
        let result = validator.validate("<script>alert('xss')</script>", "general").unwrap();
        assert!(!result.is_valid);
        assert_eq!(result.errors[0].rule_name, "xss_protection");

        // Test password validation
        let result = validator.validate("weakpass", "password").unwrap();
        assert!(!result.is_valid);

        let result = validator.validate("StrongP@ssw0rd123!", "password").unwrap();
        assert!(result.is_valid);
    }

    #[test]
    fn test_input_sanitization() {
        let config = ValidationConfig::default();
        let validator = InputValidator::new(config).unwrap();

        let input = "<script>alert('xss')</script>Hello World!";
        let sanitized = validator.sanitize_input(input).unwrap();
        
        // Should not contain script tags
        assert!(!sanitized.contains("<script>"));
        assert!(sanitized.contains("Hello World!"));
    }
}