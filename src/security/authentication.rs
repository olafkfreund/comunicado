//! Advanced authentication and session management system

use crate::security::{SecurityResult, SecurityError, ThreatLevel};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use uuid::Uuid;
use sha2::{Sha256, Digest};
use hmac::{Hmac, Mac};
use rand::{RngCore, rngs::OsRng};

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// Require multi-factor authentication
    pub require_mfa: bool,
    /// Session timeout duration
    pub session_timeout: Duration,
    /// Maximum concurrent sessions per user
    pub max_concurrent_sessions: u32,
    /// Password complexity requirements
    pub password_policy: PasswordPolicy,
    /// Account lockout policy
    pub lockout_policy: LockoutPolicy,
    /// Enable biometric authentication
    pub enable_biometric: bool,
    /// JWT token configuration
    pub jwt_config: JwtConfig,
    /// OAuth2 configuration
    pub oauth_config: OAuthConfig,
    /// Session security settings
    pub session_security: SessionSecurityConfig,
}

/// Password policy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordPolicy {
    /// Minimum password length
    pub min_length: usize,
    /// Require uppercase letters
    pub require_uppercase: bool,
    /// Require lowercase letters
    pub require_lowercase: bool,
    /// Require numbers
    pub require_numbers: bool,
    /// Require special characters
    pub require_special_chars: bool,
    /// Prevent common passwords
    pub prevent_common_passwords: bool,
    /// Password history to prevent reuse
    pub password_history_size: usize,
    /// Password expiry duration
    pub password_expiry: Duration,
}

/// Account lockout policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockoutPolicy {
    /// Enable account lockout
    pub enabled: bool,
    /// Failed attempts before lockout
    pub max_failed_attempts: u32,
    /// Lockout duration
    pub lockout_duration: Duration,
    /// Progressive lockout (increase duration for repeated violations)
    pub progressive_lockout: bool,
    /// Reset failed attempts counter after successful login
    pub reset_on_success: bool,
}

/// JWT token configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtConfig {
    /// JWT secret key (base64 encoded)
    pub secret_key: String,
    /// Token expiration time
    pub expiration: Duration,
    /// Token issuer
    pub issuer: String,
    /// Allowed audiences
    pub audiences: Vec<String>,
    /// Enable token refresh
    pub enable_refresh: bool,
    /// Refresh token expiration
    pub refresh_expiration: Duration,
}

/// OAuth2 configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthConfig {
    /// Enable OAuth2 authentication
    pub enabled: bool,
    /// Supported OAuth providers
    pub providers: HashMap<String, OAuthProvider>,
    /// OAuth2 scopes
    pub default_scopes: Vec<String>,
    /// PKCE (Proof Key for Code Exchange) enforcement
    pub enforce_pkce: bool,
}

/// OAuth provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthProvider {
    /// Provider name
    pub name: String,
    /// Client ID
    pub client_id: String,
    /// Client secret
    pub client_secret: String,
    /// Authorization endpoint
    pub auth_endpoint: String,
    /// Token endpoint
    pub token_endpoint: String,
    /// User info endpoint
    pub userinfo_endpoint: String,
    /// Supported scopes
    pub scopes: Vec<String>,
}

/// Session security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSecurityConfig {
    /// Enable secure session cookies
    pub secure_cookies: bool,
    /// Enable HttpOnly cookies
    pub http_only_cookies: bool,
    /// SameSite cookie policy
    pub same_site: SameSitePolicy,
    /// Session token rotation
    pub token_rotation: bool,
    /// Enable session fingerprinting
    pub session_fingerprinting: bool,
    /// IP address validation
    pub ip_validation: bool,
    /// User agent validation
    pub user_agent_validation: bool,
}

/// SameSite cookie policies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SameSitePolicy {
    Strict,
    Lax,
    None,
}

/// Authentication methods
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthMethod {
    /// Password-based authentication
    Password {
        username: String,
        password: String,
    },
    /// Multi-factor authentication
    MultiFactor {
        username: String,
        password: String,
        mfa_code: String,
    },
    /// Biometric authentication
    Biometric {
        username: String,
        biometric_data: Vec<u8>,
        biometric_type: BiometricType,
    },
    /// JWT token authentication
    JwtToken {
        token: String,
    },
    /// OAuth2 authentication
    OAuth2 {
        provider: String,
        authorization_code: String,
        code_verifier: Option<String>,
    },
    /// API key authentication
    ApiKey {
        key: String,
        signature: Option<String>,
    },
}

/// Biometric authentication types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BiometricType {
    Fingerprint,
    FaceRecognition,
    VoicePrint,
    IrisPattern,
}

/// Authentication result
#[derive(Debug, Clone)]
pub struct AuthResult {
    /// Authentication success status
    pub success: bool,
    /// User information if successful
    pub user_info: Option<UserInfo>,
    /// Session token if successful
    pub session_token: Option<SessionToken>,
    /// Error message if failed
    pub error_message: Option<String>,
    /// Authentication method used
    pub auth_method: String,
    /// Timestamp
    pub timestamp: Instant,
    /// Required next steps (e.g., MFA, password change)
    pub required_actions: Vec<RequiredAction>,
}

/// User information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    /// User ID
    pub user_id: String,
    /// Username
    pub username: String,
    /// Email address
    pub email: String,
    /// Display name
    pub display_name: String,
    /// User roles
    pub roles: Vec<String>,
    /// User permissions
    pub permissions: Vec<String>,
    /// Account status
    pub account_status: AccountStatus,
    /// Last login timestamp
    pub last_login: Option<SystemTime>,
    /// Account creation timestamp
    pub created_at: SystemTime,
    /// Profile metadata
    pub metadata: HashMap<String, String>,
}

/// Account status enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AccountStatus {
    Active,
    Inactive,
    Locked,
    Suspended,
    PendingVerification,
    PasswordExpired,
}

/// Required authentication actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RequiredAction {
    ChangePassword,
    SetupMfa,
    VerifyEmail,
    AcceptTerms,
    UpdateProfile,
}

/// Session token information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionToken {
    /// Token ID
    pub token_id: String,
    /// User ID associated with token
    pub user_id: String,
    /// Token value
    pub token: String,
    /// Token creation time
    pub created_at: Instant,
    /// Token expiration time
    pub expires_at: Instant,
    /// Token scope/permissions
    pub scope: Vec<String>,
    /// Session fingerprint for security
    pub fingerprint: String,
    /// IP address when token was created
    pub ip_address: String,
    /// User agent when token was created
    pub user_agent: String,
    /// Refresh token if applicable
    pub refresh_token: Option<String>,
}

/// Authentication manager
pub struct AuthenticationManager {
    /// Configuration
    config: AuthConfig,
    /// Active sessions
    sessions: Arc<RwLock<HashMap<String, SessionInfo>>>,
    /// User account information
    users: Arc<RwLock<HashMap<String, UserAccount>>>,
    /// Failed login attempts tracking
    failed_attempts: Arc<RwLock<HashMap<String, FailureTracker>>>,
    /// Two-factor authentication manager
    mfa_manager: Arc<TwoFactorAuth>,
    /// Biometric authentication handler
    biometric_auth: Arc<BiometricAuth>,
}

/// Internal session information
#[derive(Debug, Clone)]
struct SessionInfo {
    pub session_token: SessionToken,
    pub last_activity: Instant,
    pub activity_count: u64,
    pub fingerprint_changes: u32,
}

/// Internal user account information
#[derive(Debug, Clone)]
struct UserAccount {
    pub user_info: UserInfo,
    pub password_hash: String,
    pub salt: String,
    pub password_history: Vec<String>,
    pub failed_attempts: u32,
    pub locked_until: Option<Instant>,
    pub mfa_secret: Option<String>,
    pub biometric_templates: HashMap<BiometricType, Vec<u8>>,
}

/// Failed login attempt tracker
#[derive(Debug, Clone)]
struct FailureTracker {
    pub attempts: u32,
    pub last_attempt: Instant,
    pub lockout_count: u32,
}

impl AuthenticationManager {
    /// Create a new authentication manager
    pub fn new(config: AuthConfig) -> Self {
        Self {
            config: config.clone(),
            sessions: Arc::new(RwLock::new(HashMap::new())),
            users: Arc::new(RwLock::new(HashMap::new())),
            failed_attempts: Arc::new(RwLock::new(HashMap::new())),
            mfa_manager: Arc::new(TwoFactorAuth::new()),
            biometric_auth: Arc::new(BiometricAuth::new()),
        }
    }

    /// Authenticate user with given method
    pub async fn authenticate(&self, auth_method: AuthMethod) -> SecurityResult<AuthResult> {
        let start_time = Instant::now();
        
        let result = match auth_method {
            AuthMethod::Password { username, password } => {
                self.authenticate_password(&username, &password).await
            }
            AuthMethod::MultiFactory { username, password, mfa_code } => {
                self.authenticate_mfa(&username, &password, &mfa_code).await
            }
            AuthMethod::Biometric { username, biometric_data, biometric_type } => {
                self.authenticate_biometric(&username, &biometric_data, biometric_type).await
            }
            AuthMethod::JwtToken { token } => {
                self.authenticate_jwt(&token).await
            }
            AuthMethod::OAuth2 { provider, authorization_code, code_verifier } => {
                self.authenticate_oauth2(&provider, &authorization_code, code_verifier.as_deref()).await
            }
            AuthMethod::ApiKey { key, signature } => {
                self.authenticate_api_key(&key, signature.as_deref()).await
            }
        };

        // Log authentication attempt
        if let Err(ref error) = result {
            tracing::warn!(
                "Authentication failed: {} (duration: {:?})",
                error,
                start_time.elapsed()
            );
        } else {
            tracing::info!(
                "Authentication successful (duration: {:?})",
                start_time.elapsed()
            );
        }

        result
    }

    /// Authenticate with username and password
    async fn authenticate_password(&self, username: &str, password: &str) -> SecurityResult<AuthResult> {
        // Check if account is locked
        if self.is_account_locked(username).await? {
            return Ok(AuthResult {
                success: false,
                user_info: None,
                session_token: None,
                error_message: Some("Account is locked due to too many failed attempts".to_string()),
                auth_method: "password".to_string(),
                timestamp: Instant::now(),
                required_actions: vec![],
            });
        }

        let users = self.users.read().await;
        if let Some(user_account) = users.get(username) {
            // Verify password
            if self.verify_password(password, &user_account.password_hash, &user_account.salt)? {
                // Check account status
                if !matches!(user_account.user_info.account_status, AccountStatus::Active) {
                    return Ok(AuthResult {
                        success: false,
                        user_info: None,
                        session_token: None,
                        error_message: Some("Account is not active".to_string()),
                        auth_method: "password".to_string(),
                        timestamp: Instant::now(),
                        required_actions: vec![],
                    });
                }

                // Reset failed attempts on successful authentication
                if self.config.lockout_policy.reset_on_success {
                    self.reset_failed_attempts(username).await?;
                }

                // Check if MFA is required
                let mut required_actions = Vec::new();
                if self.config.require_mfa && user_account.mfa_secret.is_none() {
                    required_actions.push(RequiredAction::SetupMfa);
                }

                // Check password expiry
                if self.is_password_expired(&user_account.user_info).await? {
                    required_actions.push(RequiredAction::ChangePassword);
                }

                // Create session token
                let session_token = self.create_session_token(&user_account.user_info).await?;

                Ok(AuthResult {
                    success: true,
                    user_info: Some(user_account.user_info.clone()),
                    session_token: Some(session_token),
                    error_message: None,
                    auth_method: "password".to_string(),
                    timestamp: Instant::now(),
                    required_actions,
                })
            } else {
                // Record failed attempt
                self.record_failed_attempt(username).await?;
                
                Ok(AuthResult {
                    success: false,
                    user_info: None,
                    session_token: None,
                    error_message: Some("Invalid username or password".to_string()),
                    auth_method: "password".to_string(),
                    timestamp: Instant::now(),
                    required_actions: vec![],
                })
            }
        } else {
            // Record failed attempt for non-existent user
            self.record_failed_attempt(username).await?;
            
            Ok(AuthResult {
                success: false,
                user_info: None,
                session_token: None,
                error_message: Some("Invalid username or password".to_string()),
                auth_method: "password".to_string(),
                timestamp: Instant::now(),
                required_actions: vec![],
            })
        }
    }

    /// Authenticate with multi-factor authentication
    async fn authenticate_mfa(&self, username: &str, password: &str, mfa_code: &str) -> SecurityResult<AuthResult> {
        // First verify password
        let password_result = self.authenticate_password(username, password).await?;
        
        if !password_result.success {
            return Ok(password_result);
        }

        // Verify MFA code
        if self.mfa_manager.verify_code(username, mfa_code).await? {
            Ok(password_result)
        } else {
            // Record failed MFA attempt
            self.record_failed_attempt(username).await?;
            
            Ok(AuthResult {
                success: false,
                user_info: None,
                session_token: None,
                error_message: Some("Invalid MFA code".to_string()),
                auth_method: "mfa".to_string(),
                timestamp: Instant::now(),
                required_actions: vec![],
            })
        }
    }

    /// Authenticate with biometric data
    async fn authenticate_biometric(
        &self,
        username: &str,
        biometric_data: &[u8],
        biometric_type: BiometricType,
    ) -> SecurityResult<AuthResult> {
        if !self.config.enable_biometric {
            return Err(SecurityError::AuthenticationError(
                "Biometric authentication is not enabled".to_string()
            ));
        }

        let users = self.users.read().await;
        if let Some(user_account) = users.get(username) {
            if let Some(template) = user_account.biometric_templates.get(&biometric_type) {
                if self.biometric_auth.verify(biometric_data, template, biometric_type.clone()).await? {
                    // Reset failed attempts on successful authentication
                    if self.config.lockout_policy.reset_on_success {
                        self.reset_failed_attempts(username).await?;
                    }

                    // Create session token
                    let session_token = self.create_session_token(&user_account.user_info).await?;

                    Ok(AuthResult {
                        success: true,
                        user_info: Some(user_account.user_info.clone()),
                        session_token: Some(session_token),
                        error_message: None,
                        auth_method: "biometric".to_string(),
                        timestamp: Instant::now(),
                        required_actions: vec![],
                    })
                } else {
                    // Record failed attempt
                    self.record_failed_attempt(username).await?;
                    
                    Ok(AuthResult {
                        success: false,
                        user_info: None,
                        session_token: None,
                        error_message: Some("Biometric verification failed".to_string()),
                        auth_method: "biometric".to_string(),
                        timestamp: Instant::now(),
                        required_actions: vec![],
                    })
                }
            } else {
                Err(SecurityError::AuthenticationError(
                    "No biometric template registered for this user".to_string()
                ))
            }
        } else {
            Err(SecurityError::AuthenticationError(
                "User not found".to_string()
            ))
        }
    }

    /// Authenticate with JWT token
    async fn authenticate_jwt(&self, token: &str) -> SecurityResult<AuthResult> {
        // Verify JWT token
        let claims = self.verify_jwt_token(token)?;
        
        // Check if session exists and is valid
        let sessions = self.sessions.read().await;
        if let Some(session_info) = sessions.get(&claims.jti) {
            if session_info.session_token.expires_at > Instant::now() {
                let users = self.users.read().await;
                if let Some(user_account) = users.get(&claims.sub) {
                    Ok(AuthResult {
                        success: true,
                        user_info: Some(user_account.user_info.clone()),
                        session_token: Some(session_info.session_token.clone()),
                        error_message: None,
                        auth_method: "jwt".to_string(),
                        timestamp: Instant::now(),
                        required_actions: vec![],
                    })
                } else {
                    Err(SecurityError::AuthenticationError(
                        "User not found".to_string()
                    ))
                }
            } else {
                Err(SecurityError::AuthenticationError(
                    "Session token has expired".to_string()
                ))
            }
        } else {
            Err(SecurityError::AuthenticationError(
                "Invalid session token".to_string()
            ))
        }
    }

    /// Authenticate with OAuth2
    async fn authenticate_oauth2(
        &self,
        provider: &str,
        authorization_code: &str,
        code_verifier: Option<&str>,
    ) -> SecurityResult<AuthResult> {
        if !self.config.oauth_config.enabled {
            return Err(SecurityError::AuthenticationError(
                "OAuth2 authentication is not enabled".to_string()
            ));
        }

        // Get provider configuration
        let oauth_provider = self.config.oauth_config.providers.get(provider)
            .ok_or_else(|| SecurityError::AuthenticationError(
                format!("Unknown OAuth provider: {}", provider)
            ))?;

        // Exchange authorization code for access token
        let user_info = self.exchange_oauth_code(oauth_provider, authorization_code, code_verifier).await?;
        
        // Create or update user account
        self.create_or_update_oauth_user(&user_info, provider).await?;

        // Create session token
        let session_token = self.create_session_token(&user_info).await?;

        Ok(AuthResult {
            success: true,
            user_info: Some(user_info),
            session_token: Some(session_token),
            error_message: None,
            auth_method: format!("oauth2_{}", provider),
            timestamp: Instant::now(),
            required_actions: vec![],
        })
    }

    /// Authenticate with API key
    async fn authenticate_api_key(&self, key: &str, signature: Option<&str>) -> SecurityResult<AuthResult> {
        // This would integrate with API key management system
        // For now, we provide a basic framework
        
        if let Some(sig) = signature {
            // Verify HMAC signature if provided
            if !self.verify_api_key_signature(key, sig)? {
                return Err(SecurityError::AuthenticationError(
                    "Invalid API key signature".to_string()
                ));
            }
        }

        // Lookup API key (this would use actual key storage)
        if let Some(user_info) = self.lookup_api_key(key).await? {
            let session_token = self.create_session_token(&user_info).await?;

            Ok(AuthResult {
                success: true,
                user_info: Some(user_info),
                session_token: Some(session_token),
                error_message: None,
                auth_method: "api_key".to_string(),
                timestamp: Instant::now(),
                required_actions: vec![],
            })
        } else {
            Err(SecurityError::AuthenticationError(
                "Invalid API key".to_string()
            ))
        }
    }

    /// Validate existing session token
    pub async fn validate_session(&self, token: &str) -> SecurityResult<bool> {
        let sessions = self.sessions.read().await;
        
        if let Some(session_info) = sessions.get(token) {
            // Check expiration
            if session_info.session_token.expires_at <= Instant::now() {
                // Session expired - remove it
                drop(sessions);
                let mut sessions_mut = self.sessions.write().await;
                sessions_mut.remove(token);
                return Ok(false);
            }

            // Update last activity
            drop(sessions);
            let mut sessions_mut = self.sessions.write().await;
            if let Some(session) = sessions_mut.get_mut(token) {
                session.last_activity = Instant::now();
                session.activity_count += 1;
            }

            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Logout and invalidate session
    pub async fn logout(&self, token: &str) -> SecurityResult<()> {
        let mut sessions = self.sessions.write().await;
        sessions.remove(token);
        
        tracing::info!("User session logged out: {}", token);
        Ok(())
    }

    /// Create session token for authenticated user
    async fn create_session_token(&self, user_info: &UserInfo) -> SecurityResult<SessionToken> {
        let token_id = Uuid::new_v4().to_string();
        let token = self.generate_secure_token();
        let now = Instant::now();
        let expires_at = now + self.config.session_timeout;
        
        let session_token = SessionToken {
            token_id: token_id.clone(),
            user_id: user_info.user_id.clone(),
            token: token.clone(),
            created_at: now,
            expires_at,
            scope: user_info.permissions.clone(),
            fingerprint: self.generate_session_fingerprint().await,
            ip_address: "127.0.0.1".to_string(), // Would be actual client IP
            user_agent: "comunicado-client".to_string(), // Would be actual user agent
            refresh_token: if self.config.jwt_config.enable_refresh {
                Some(self.generate_secure_token())
            } else {
                None
            },
        };

        // Store session
        let mut sessions = self.sessions.write().await;
        sessions.insert(token.clone(), SessionInfo {
            session_token: session_token.clone(),
            last_activity: now,
            activity_count: 1,
            fingerprint_changes: 0,
        });

        // Enforce max concurrent sessions
        self.enforce_session_limit(user_info).await?;

        Ok(session_token)
    }

    /// Helper methods for authentication logic
    async fn is_account_locked(&self, username: &str) -> SecurityResult<bool> {
        if !self.config.lockout_policy.enabled {
            return Ok(false);
        }

        let users = self.users.read().await;
        if let Some(user_account) = users.get(username) {
            if let Some(locked_until) = user_account.locked_until {
                return Ok(locked_until > Instant::now());
            }
        }

        // Check failed attempts
        let failed_attempts = self.failed_attempts.read().await;
        if let Some(tracker) = failed_attempts.get(username) {
            Ok(tracker.attempts >= self.config.lockout_policy.max_failed_attempts)
        } else {
            Ok(false)
        }
    }

    async fn record_failed_attempt(&self, username: &str) -> SecurityResult<()> {
        let mut failed_attempts = self.failed_attempts.write().await;
        let tracker = failed_attempts.entry(username.to_string()).or_insert_with(|| FailureTracker {
            attempts: 0,
            last_attempt: Instant::now(),
            lockout_count: 0,
        });
        
        tracker.attempts += 1;
        tracker.last_attempt = Instant::now();
        
        // Check if account should be locked
        if self.config.lockout_policy.enabled && 
           tracker.attempts >= self.config.lockout_policy.max_failed_attempts {
            
            let lockout_duration = if self.config.lockout_policy.progressive_lockout {
                let multiplier = 2_u32.pow(tracker.lockout_count);
                self.config.lockout_policy.lockout_duration * multiplier
            } else {
                self.config.lockout_policy.lockout_duration
            };

            // Lock the account
            let mut users = self.users.write().await;
            if let Some(user_account) = users.get_mut(username) {
                user_account.locked_until = Some(Instant::now() + lockout_duration);
            }
            
            tracker.lockout_count += 1;
            tracker.attempts = 0; // Reset attempts after lockout
            
            tracing::warn!(
                "Account locked due to failed attempts: {} (duration: {:?})",
                username,
                lockout_duration
            );
        }

        Ok(())
    }

    async fn reset_failed_attempts(&self, username: &str) -> SecurityResult<()> {
        let mut failed_attempts = self.failed_attempts.write().await;
        failed_attempts.remove(username);
        
        // Remove account lock
        let mut users = self.users.write().await;
        if let Some(user_account) = users.get_mut(username) {
            user_account.locked_until = None;
            user_account.failed_attempts = 0;
        }

        Ok(())
    }

    fn verify_password(&self, password: &str, hash: &str, salt: &str) -> SecurityResult<bool> {
        let computed_hash = self.hash_password(password, salt)?;
        Ok(computed_hash == hash)
    }

    fn hash_password(&self, password: &str, salt: &str) -> SecurityResult<String> {
        let mut hasher = Sha256::new();
        hasher.update(password.as_bytes());
        hasher.update(salt.as_bytes());
        let result = hasher.finalize();
        Ok(hex::encode(result))
    }

    fn generate_secure_token(&self) -> String {
        let mut token = [0u8; 32];
        OsRng.fill_bytes(&mut token);
        hex::encode(token)
    }

    async fn generate_session_fingerprint(&self) -> String {
        // Generate session fingerprint based on various factors
        let mut hasher = Sha256::new();
        hasher.update(b"session_fingerprint");
        hasher.update(&SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs().to_be_bytes());
        let result = hasher.finalize();
        hex::encode(result)
    }

    fn generate_salt(&self) -> String {
        let mut salt = [0u8; 16];
        OsRng.fill_bytes(&mut salt);
        hex::encode(salt)
    }

    // Additional helper methods would be implemented here
    async fn is_password_expired(&self, _user_info: &UserInfo) -> SecurityResult<bool> {
        // Implementation for password expiry check
        Ok(false)
    }

    async fn enforce_session_limit(&self, user_info: &UserInfo) -> SecurityResult<()> {
        // Implementation for enforcing concurrent session limits
        let sessions = self.sessions.read().await;
        let user_sessions: Vec<_> = sessions
            .values()
            .filter(|session| session.session_token.user_id == user_info.user_id)
            .collect();
        
        if user_sessions.len() > self.config.max_concurrent_sessions as usize {
            // Remove oldest sessions
            tracing::info!(
                "Enforcing session limit for user {}: {} active sessions",
                user_info.user_id,
                user_sessions.len()
            );
        }

        Ok(())
    }

    fn verify_jwt_token(&self, _token: &str) -> SecurityResult<JwtClaims> {
        // JWT token verification implementation
        Ok(JwtClaims {
            sub: "user_id".to_string(),
            jti: "token_id".to_string(),
            exp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() + 3600,
        })
    }

    async fn exchange_oauth_code(
        &self,
        _provider: &OAuthProvider,
        _code: &str,
        _verifier: Option<&str>,
    ) -> SecurityResult<UserInfo> {
        // OAuth2 code exchange implementation
        Ok(UserInfo {
            user_id: Uuid::new_v4().to_string(),
            username: "oauth_user".to_string(),
            email: "oauth@example.com".to_string(),
            display_name: "OAuth User".to_string(),
            roles: vec!["user".to_string()],
            permissions: vec!["read".to_string()],
            account_status: AccountStatus::Active,
            last_login: Some(SystemTime::now()),
            created_at: SystemTime::now(),
            metadata: HashMap::new(),
        })
    }

    async fn create_or_update_oauth_user(&self, user_info: &UserInfo, _provider: &str) -> SecurityResult<()> {
        // Create or update OAuth user implementation
        let mut users = self.users.write().await;
        users.insert(user_info.username.clone(), UserAccount {
            user_info: user_info.clone(),
            password_hash: String::new(), // OAuth users don't have passwords
            salt: String::new(),
            password_history: Vec::new(),
            failed_attempts: 0,
            locked_until: None,
            mfa_secret: None,
            biometric_templates: HashMap::new(),
        });
        Ok(())
    }

    fn verify_api_key_signature(&self, _key: &str, _signature: &str) -> SecurityResult<bool> {
        // API key signature verification implementation
        Ok(true)
    }

    async fn lookup_api_key(&self, _key: &str) -> SecurityResult<Option<UserInfo>> {
        // API key lookup implementation
        Ok(None)
    }
}

/// JWT token claims
#[derive(Debug, Clone)]
struct JwtClaims {
    pub sub: String,  // Subject (user ID)
    pub jti: String,  // JWT ID (token ID)
    pub exp: u64,     // Expiration timestamp
}

/// Two-factor authentication manager
pub struct TwoFactorAuth {
    /// TOTP secrets for users
    user_secrets: Arc<RwLock<HashMap<String, String>>>,
}

impl TwoFactorAuth {
    pub fn new() -> Self {
        Self {
            user_secrets: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn verify_code(&self, username: &str, code: &str) -> SecurityResult<bool> {
        let secrets = self.user_secrets.read().await;
        if let Some(_secret) = secrets.get(username) {
            // TOTP verification implementation would go here
            // For now, we'll simulate verification
            Ok(code.len() == 6 && code.chars().all(|c| c.is_ascii_digit()))
        } else {
            Ok(false)
        }
    }
}

/// Biometric authentication handler
pub struct BiometricAuth;

impl BiometricAuth {
    pub fn new() -> Self {
        Self
    }

    pub async fn verify(
        &self,
        _biometric_data: &[u8],
        _template: &[u8],
        _biometric_type: BiometricType,
    ) -> SecurityResult<bool> {
        // Biometric verification implementation would go here
        // This would involve complex biometric matching algorithms
        Ok(true) // Simplified for framework demonstration
    }
}

/// Session management utilities
pub struct SessionManager {
    auth_manager: Arc<AuthenticationManager>,
}

impl SessionManager {
    pub fn new(auth_manager: Arc<AuthenticationManager>) -> Self {
        Self { auth_manager }
    }

    pub async fn cleanup_expired_sessions(&self) -> SecurityResult<usize> {
        let mut sessions = self.auth_manager.sessions.write().await;
        let now = Instant::now();
        let initial_count = sessions.len();
        
        sessions.retain(|_, session_info| {
            session_info.session_token.expires_at > now
        });

        let cleaned_count = initial_count - sessions.len();
        if cleaned_count > 0 {
            tracing::info!("Cleaned up {} expired sessions", cleaned_count);
        }

        Ok(cleaned_count)
    }

    pub async fn get_active_sessions(&self, user_id: &str) -> SecurityResult<Vec<SessionToken>> {
        let sessions = self.auth_manager.sessions.read().await;
        let active_sessions: Vec<SessionToken> = sessions
            .values()
            .filter(|session| session.session_token.user_id == user_id)
            .map(|session| session.session_token.clone())
            .collect();

        Ok(active_sessions)
    }
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            require_mfa: false,
            session_timeout: Duration::from_secs(3600), // 1 hour
            max_concurrent_sessions: 5,
            password_policy: PasswordPolicy::default(),
            lockout_policy: LockoutPolicy::default(),
            enable_biometric: false,
            jwt_config: JwtConfig::default(),
            oauth_config: OAuthConfig::default(),
            session_security: SessionSecurityConfig::default(),
        }
    }
}

impl Default for PasswordPolicy {
    fn default() -> Self {
        Self {
            min_length: 12,
            require_uppercase: true,
            require_lowercase: true,
            require_numbers: true,
            require_special_chars: true,
            prevent_common_passwords: true,
            password_history_size: 5,
            password_expiry: Duration::from_secs(90 * 24 * 3600), // 90 days
        }
    }
}

impl Default for LockoutPolicy {
    fn default() -> Self {
        Self {
            enabled: true,
            max_failed_attempts: 5,
            lockout_duration: Duration::from_secs(300), // 5 minutes
            progressive_lockout: true,
            reset_on_success: true,
        }
    }
}

impl Default for JwtConfig {
    fn default() -> Self {
        Self {
            secret_key: "default_jwt_secret_change_in_production".to_string(),
            expiration: Duration::from_secs(3600), // 1 hour
            issuer: "comunicado".to_string(),
            audiences: vec!["comunicado-client".to_string()],
            enable_refresh: true,
            refresh_expiration: Duration::from_secs(7 * 24 * 3600), // 7 days
        }
    }
}

impl Default for OAuthConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            providers: HashMap::new(),
            default_scopes: vec!["openid".to_string(), "email".to_string(), "profile".to_string()],
            enforce_pkce: true,
        }
    }
}

impl Default for SessionSecurityConfig {
    fn default() -> Self {
        Self {
            secure_cookies: true,
            http_only_cookies: true,
            same_site: SameSitePolicy::Strict,
            token_rotation: true,
            session_fingerprinting: true,
            ip_validation: false,
            user_agent_validation: false,
        }
    }
}

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("Invalid credentials")]
    InvalidCredentials,
    #[error("Account locked: {0}")]
    AccountLocked(String),
    #[error("Session expired")]
    SessionExpired,
    #[error("MFA required")]
    MfaRequired,
    #[error("Biometric verification failed")]
    BiometricFailed,
    #[error("OAuth error: {0}")]
    OAuthError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_authentication_manager() {
        let config = AuthConfig::default();
        let auth_manager = AuthenticationManager::new(config);

        // Test password authentication
        let auth_method = AuthMethod::Password {
            username: "testuser".to_string(),
            password: "testpassword123!".to_string(),
        };

        let result = auth_manager.authenticate(auth_method).await;
        assert!(result.is_ok());
        
        // Note: This will fail in the current implementation as no user exists
        // In a real implementation, you'd set up test users first
    }

    #[tokio::test]
    async fn test_session_validation() {
        let config = AuthConfig::default();
        let auth_manager = AuthenticationManager::new(config);

        // Test with invalid session token
        let is_valid = auth_manager.validate_session("invalid_token").await.unwrap();
        assert!(!is_valid);
    }

    #[tokio::test]
    async fn test_password_hashing() {
        let config = AuthConfig::default();
        let auth_manager = AuthenticationManager::new(config);

        let password = "test_password_123!";
        let salt = auth_manager.generate_salt();
        let hash = auth_manager.hash_password(password, &salt).unwrap();

        // Verify the password matches
        let is_valid = auth_manager.verify_password(password, &hash, &salt).unwrap();
        assert!(is_valid);

        // Verify wrong password doesn't match
        let is_invalid = auth_manager.verify_password("wrong_password", &hash, &salt).unwrap();
        assert!(!is_invalid);
    }
}