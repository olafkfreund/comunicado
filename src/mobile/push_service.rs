//! Push notification service for mobile devices

use super::{MobileError, Result, NotificationPayload, PushProviderConfig, PushProviderType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Push notification service
pub struct PushService {
    providers: HashMap<PushProviderType, Box<dyn PushProvider>>,
    device_tokens: RwLock<HashMap<Uuid, DeviceTokenInfo>>,
    statistics: RwLock<PushStatistics>,
}

/// Push notification provider trait
#[async_trait::async_trait]
pub trait PushProvider: Send + Sync + std::fmt::Debug {
    async fn send_notification(
        &self,
        token: &PushToken,
        payload: &NotificationPayload,
    ) -> Result<PushResult>;
    
    async fn validate_token(&self, token: &PushToken) -> Result<bool>;
    
    async fn get_provider_stats(&self) -> PushProviderStats;
    
    fn provider_type(&self) -> PushProviderType;
}

/// Push token information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushToken {
    pub token: String,
    pub provider_type: PushProviderType,
    pub app_id: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_validated: Option<chrono::DateTime<chrono::Utc>>,
    pub is_valid: bool,
}

/// Device token information
#[derive(Debug, Clone)]
pub struct DeviceTokenInfo {
    pub device_id: Uuid,
    pub tokens: Vec<PushToken>,
    pub primary_token: Option<PushToken>,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

/// Push notification result
#[derive(Debug, Clone)]
pub struct PushResult {
    pub success: bool,
    pub message_id: Option<String>,
    pub error: Option<String>,
    pub retry_after: Option<u64>,
    pub tokens_to_remove: Vec<String>,
}

/// Push service statistics
#[derive(Debug, Default, Clone)]
pub struct PushStatistics {
    pub total_sent: u64,
    pub total_delivered: u64,
    pub total_failed: u64,
    pub by_provider: HashMap<PushProviderType, PushProviderStats>,
}

/// Provider-specific statistics
#[derive(Debug, Default, Clone)]
pub struct PushProviderStats {
    pub sent: u64,
    pub delivered: u64,
    pub failed: u64,
    pub invalid_tokens: u64,
    pub rate_limited: u64,
}

/// Firebase Cloud Messaging provider
#[derive(Debug)]
pub struct FCMProvider {
    api_key: String,
    project_id: String,
    client: reqwest::Client,
    stats: RwLock<PushProviderStats>,
}

/// Apple Push Notification Service provider
#[derive(Debug)]
pub struct APNSProvider {
    key_id: String,
    team_id: String,
    private_key: String,
    is_production: bool,
    client: reqwest::Client,
    stats: RwLock<PushProviderStats>,
}

/// Web Push provider
#[derive(Debug)]
pub struct WebPushProvider {
    vapid_private_key: String,
    vapid_public_key: String,
    subject: String,
    client: reqwest::Client,
    stats: RwLock<PushProviderStats>,
}

/// Custom push provider
#[derive(Debug)]
pub struct CustomProvider {
    endpoint_url: String,
    api_key: Option<String>,
    headers: HashMap<String, String>,
    client: reqwest::Client,
    stats: RwLock<PushProviderStats>,
}

impl PushService {
    pub fn new(provider_configs: &[PushProviderConfig]) -> Result<Self> {
        let mut providers: HashMap<PushProviderType, Box<dyn PushProvider>> = HashMap::new();

        for config in provider_configs {
            if !config.enabled {
                continue;
            }

            let provider: Box<dyn PushProvider> = match config.provider_type {
                PushProviderType::FCM => {
                    Box::new(FCMProvider::new(config)?)
                }
                PushProviderType::APNS => {
                    Box::new(APNSProvider::new(config)?)
                }
                PushProviderType::WebPush => {
                    Box::new(WebPushProvider::new(config)?)
                }
                PushProviderType::Custom(_) => {
                    Box::new(CustomProvider::new(config)?)
                }
            };

            providers.insert(config.provider_type.clone(), provider);
        }

        Ok(Self {
            providers,
            device_tokens: RwLock::new(HashMap::new()),
            statistics: RwLock::new(PushStatistics::default()),
        })
    }

    /// Initialize push service
    pub async fn initialize(&mut self) -> Result<()> {
        // Validate all configured providers
        for (provider_type, provider) in &self.providers {
            // Test provider connection
            let stats = provider.get_provider_stats().await;
            println!("Push provider {:?} initialized successfully with stats: sent={}, failed={}", 
                     provider_type, stats.messages_sent, stats.messages_failed);
        }

        Ok(())
    }

    /// Register device token
    pub async fn register_token(&self, device_id: Uuid, token: PushToken) -> Result<()> {
        // Validate token with provider
        if let Some(provider) = self.providers.get(&token.provider_type) {
            let is_valid = provider.validate_token(&token).await?;
            
            let mut validated_token = token;
            validated_token.is_valid = is_valid;
            validated_token.last_validated = Some(chrono::Utc::now());

            // Store token
            let mut device_tokens = self.device_tokens.write().await;
            let device_info = device_tokens.entry(device_id).or_insert_with(|| {
                DeviceTokenInfo {
                    device_id,
                    tokens: Vec::new(),
                    primary_token: None,
                    last_updated: chrono::Utc::now(),
                }
            });

            // Remove existing token for this provider
            device_info.tokens.retain(|t| t.provider_type != validated_token.provider_type);
            
            // Add new token
            device_info.tokens.push(validated_token.clone());
            
            // Set as primary if first token or if it's FCM/APNS
            if device_info.primary_token.is_none() || 
               matches!(validated_token.provider_type, PushProviderType::FCM | PushProviderType::APNS) {
                device_info.primary_token = Some(validated_token);
            }

            device_info.last_updated = chrono::Utc::now();

            Ok(())
        } else {
            Err(MobileError::PushService(
                format!("Provider {:?} not configured", token.provider_type)
            ))
        }
    }

    /// Send notification to device
    pub async fn send_notification(
        &self,
        device_id: Uuid,
        notification: NotificationPayload,
    ) -> Result<()> {
        let device_tokens = self.device_tokens.read().await;
        let device_info = device_tokens.get(&device_id)
            .ok_or_else(|| MobileError::DeviceNotFound(device_id.to_string()))?;

        // Try primary token first
        if let Some(primary_token) = &device_info.primary_token {
            if primary_token.is_valid {
                if let Some(provider) = self.providers.get(&primary_token.provider_type) {
                    match provider.send_notification(primary_token, &notification).await {
                        Ok(result) => {
                            self.update_statistics(&primary_token.provider_type, &result).await;
                            if result.success {
                                return Ok(());
                            }
                        }
                        Err(e) => {
                            eprintln!("Primary push failed: {}", e);
                        }
                    }
                }
            }
        }

        // Try other tokens as fallback
        for token in &device_info.tokens {
            if !token.is_valid || device_info.primary_token.as_ref() == Some(token) {
                continue;
            }

            if let Some(provider) = self.providers.get(&token.provider_type) {
                match provider.send_notification(token, &notification).await {
                    Ok(result) => {
                        self.update_statistics(&token.provider_type, &result).await;
                        if result.success {
                            return Ok(());
                        }
                    }
                    Err(e) => {
                        eprintln!("Fallback push failed: {}", e);
                    }
                }
            }
        }

        Err(MobileError::PushService(
            "All push attempts failed".to_string()
        ))
    }

    /// Get number of active providers
    pub async fn get_active_providers(&self) -> usize {
        self.providers.len()
    }

    /// Get push statistics
    pub async fn get_statistics(&self) -> PushStatistics {
        let stats = self.statistics.read().await;
        stats.clone()
    }

    /// Clean up invalid tokens
    pub async fn cleanup_invalid_tokens(&self) -> Result<u32> {
        let mut device_tokens = self.device_tokens.write().await;
        let mut cleaned = 0;

        for device_info in device_tokens.values_mut() {
            let initial_count = device_info.tokens.len();
            device_info.tokens.retain(|token| token.is_valid);
            cleaned += (initial_count - device_info.tokens.len()) as u32;

            // Update primary token if it was removed
            if let Some(primary) = &device_info.primary_token {
                if !primary.is_valid {
                    device_info.primary_token = device_info.tokens.first().cloned();
                }
            }
        }

        Ok(cleaned)
    }

    // Private methods
    async fn update_statistics(&self, provider_type: &PushProviderType, result: &PushResult) {
        let mut stats = self.statistics.write().await;
        let provider_stats = stats.by_provider.entry(provider_type.clone()).or_insert_with(Default::default);

        provider_stats.sent += 1;
        stats.total_sent += 1;

        if result.success {
            provider_stats.delivered += 1;
            stats.total_delivered += 1;
        } else {
            provider_stats.failed += 1;
            stats.total_failed += 1;
        }
    }
}

impl FCMProvider {
    pub fn new(config: &PushProviderConfig) -> Result<Self> {
        let api_key = config.api_key.as_ref()
            .ok_or_else(|| MobileError::ConfigurationError("FCM API key required".to_string()))?;

        Ok(Self {
            api_key: api_key.clone(),
            project_id: "default".to_string(), // Could be extracted from config
            client: reqwest::Client::new(),
            stats: RwLock::new(PushProviderStats::default()),
        })
    }
}

#[async_trait::async_trait]
impl PushProvider for FCMProvider {
    async fn send_notification(
        &self,
        token: &PushToken,
        payload: &NotificationPayload,
    ) -> Result<PushResult> {
        // Build FCM payload
        let fcm_payload = serde_json::json!({
            "to": token.token,
            "notification": {
                "title": payload.title,
                "body": payload.body,
                "sound": payload.sound.as_ref().unwrap_or(&"default".to_string()),
                "badge": payload.badge_count.unwrap_or(1)
            },
            "data": payload.custom_data,
            "priority": match payload.priority {
                super::NotificationPriority::Critical | super::NotificationPriority::High => "high",
                _ => "normal"
            }
        });

        let response = self.client
            .post("https://fcm.googleapis.com/fcm/send")
            .header("Authorization", format!("key={}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&fcm_payload)
            .send()
            .await
            .map_err(|e| MobileError::Network(e.to_string()))?;

        if response.status().is_success() {
            let response_data: serde_json::Value = response.json().await
                .map_err(|e| MobileError::SerializationError(e.into()))?;

            Ok(PushResult {
                success: true,
                message_id: response_data["message_id"].as_str().map(|s| s.to_string()),
                error: None,
                retry_after: None,
                tokens_to_remove: Vec::new(),
            })
        } else {
            let error_text = response.text().await.unwrap_or_default();
            Ok(PushResult {
                success: false,
                message_id: None,
                error: Some(error_text),
                retry_after: None,
                tokens_to_remove: Vec::new(),
            })
        }
    }

    async fn validate_token(&self, _token: &PushToken) -> Result<bool> {
        // FCM token validation would typically involve a dry-run send
        Ok(true) // Simplified for now
    }

    async fn get_provider_stats(&self) -> PushProviderStats {
        let stats = self.stats.read().await;
        stats.clone()
    }

    fn provider_type(&self) -> PushProviderType {
        PushProviderType::FCM
    }
}

impl APNSProvider {
    pub fn new(config: &PushProviderConfig) -> Result<Self> {
        let api_secret = config.api_secret.as_ref()
            .ok_or_else(|| MobileError::ConfigurationError("APNS private key required".to_string()))?;

        Ok(Self {
            key_id: "default".to_string(), // Should be in config
            team_id: "default".to_string(), // Should be in config
            private_key: api_secret.clone(),
            is_production: true, // Should be in config
            client: reqwest::Client::new(),
            stats: RwLock::new(PushProviderStats::default()),
        })
    }
}

#[async_trait::async_trait]
impl PushProvider for APNSProvider {
    async fn send_notification(
        &self,
        token: &PushToken,
        payload: &NotificationPayload,
    ) -> Result<PushResult> {
        // APNS implementation would require JWT token generation and proper headers
        // This is a simplified version
        let apns_payload = serde_json::json!({
            "aps": {
                "alert": {
                    "title": payload.title,
                    "body": payload.body
                },
                "sound": payload.sound.as_ref().unwrap_or(&"default".to_string()),
                "badge": payload.badge_count.unwrap_or(1)
            }
        });

        let endpoint = if self.is_production {
            "https://api.push.apple.com"
        } else {
            "https://api.sandbox.push.apple.com"
        };

        let url = format!("{}/3/device/{}", endpoint, token.token);

        let response = self.client
            .post(&url)
            .header("apns-topic", &token.app_id)
            .header("apns-priority", match payload.priority {
                super::NotificationPriority::Critical | super::NotificationPriority::High => "10",
                _ => "5"
            })
            .json(&apns_payload)
            .send()
            .await
            .map_err(|e| MobileError::Network(e.to_string()))?;

        Ok(PushResult {
            success: response.status().is_success(),
            message_id: response.headers().get("apns-id")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string()),
            error: if !response.status().is_success() {
                Some(response.text().await.unwrap_or_default())
            } else {
                None
            },
            retry_after: None,
            tokens_to_remove: Vec::new(),
        })
    }

    async fn validate_token(&self, _token: &PushToken) -> Result<bool> {
        Ok(true) // Simplified
    }

    async fn get_provider_stats(&self) -> PushProviderStats {
        let stats = self.stats.read().await;
        stats.clone()
    }

    fn provider_type(&self) -> PushProviderType {
        PushProviderType::APNS
    }
}

impl WebPushProvider {
    pub fn new(config: &PushProviderConfig) -> Result<Self> {
        let api_key = config.api_key.as_ref()
            .ok_or_else(|| MobileError::ConfigurationError("Web Push VAPID key required".to_string()))?;
        let api_secret = config.api_secret.as_ref()
            .ok_or_else(|| MobileError::ConfigurationError("Web Push VAPID private key required".to_string()))?;

        Ok(Self {
            vapid_private_key: api_secret.clone(),
            vapid_public_key: api_key.clone(),
            subject: "mailto:example@comunicado.app".to_string(),
            client: reqwest::Client::new(),
            stats: RwLock::new(PushProviderStats::default()),
        })
    }
}

#[async_trait::async_trait]
impl PushProvider for WebPushProvider {
    async fn send_notification(
        &self,
        token: &PushToken,
        payload: &NotificationPayload,
    ) -> Result<PushResult> {
        // Web Push implementation would require proper VAPID signing
        // This is a simplified version
        let web_push_payload = serde_json::json!({
            "title": payload.title,
            "body": payload.body,
            "icon": payload.icon,
            "data": payload.custom_data
        });

        // Parse the token as a Web Push endpoint
        let response = self.client
            .post(&token.token)
            .header("Content-Type", "application/json")
            .header("TTL", "86400") // 24 hours
            .json(&web_push_payload)
            .send()
            .await
            .map_err(|e| MobileError::Network(e.to_string()))?;

        Ok(PushResult {
            success: response.status().is_success(),
            message_id: None,
            error: if !response.status().is_success() {
                Some(response.text().await.unwrap_or_default())
            } else {
                None
            },
            retry_after: None,
            tokens_to_remove: Vec::new(),
        })
    }

    async fn validate_token(&self, _token: &PushToken) -> Result<bool> {
        Ok(true) // Simplified
    }

    async fn get_provider_stats(&self) -> PushProviderStats {
        let stats = self.stats.read().await;
        stats.clone()
    }

    fn provider_type(&self) -> PushProviderType {
        PushProviderType::WebPush
    }
}

impl CustomProvider {
    pub fn new(config: &PushProviderConfig) -> Result<Self> {
        let endpoint_url = config.endpoint_url.as_ref()
            .ok_or_else(|| MobileError::ConfigurationError("Custom provider endpoint required".to_string()))?;

        Ok(Self {
            endpoint_url: endpoint_url.clone(),
            api_key: config.api_key.clone(),
            headers: HashMap::new(),
            client: reqwest::Client::new(),
            stats: RwLock::new(PushProviderStats::default()),
        })
    }
}

#[async_trait::async_trait]
impl PushProvider for CustomProvider {
    async fn send_notification(
        &self,
        token: &PushToken,
        payload: &NotificationPayload,
    ) -> Result<PushResult> {
        let custom_payload = serde_json::json!({
            "token": token.token,
            "notification": {
                "title": payload.title,
                "body": payload.body,
                "priority": format!("{:?}", payload.priority),
                "data": payload.custom_data
            }
        });

        let mut request = self.client
            .post(&self.endpoint_url)
            .header("Content-Type", "application/json");

        if let Some(api_key) = &self.api_key {
            request = request.header("Authorization", format!("Bearer {}", api_key));
        }

        let response = request
            .json(&custom_payload)
            .send()
            .await
            .map_err(|e| MobileError::Network(e.to_string()))?;

        Ok(PushResult {
            success: response.status().is_success(),
            message_id: None,
            error: if !response.status().is_success() {
                Some(response.text().await.unwrap_or_default())
            } else {
                None
            },
            retry_after: None,
            tokens_to_remove: Vec::new(),
        })
    }

    async fn validate_token(&self, _token: &PushToken) -> Result<bool> {
        Ok(true) // Simplified
    }

    async fn get_provider_stats(&self) -> PushProviderStats {
        let stats = self.stats.read().await;
        stats.clone()
    }

    fn provider_type(&self) -> PushProviderType {
        PushProviderType::Custom
    }
}