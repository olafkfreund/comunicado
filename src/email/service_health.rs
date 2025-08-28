//! Service health monitoring and graceful degradation
//!
//! This module provides health monitoring for various services and implements graceful
//! degradation strategies when services become unavailable.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, warn, error, info};

/// Health status of a service
#[derive(Debug, Clone, PartialEq)]
pub enum ServiceHealth {
    /// Service is healthy and fully operational
    Healthy,
    /// Service is degraded but still functional
    Degraded { reason: String },
    /// Service is unhealthy and should not be used
    Unhealthy { reason: String, since: Instant },
    /// Service status is unknown (not yet checked)
    Unknown,
}

impl ServiceHealth {
    pub fn is_available(&self) -> bool {
        matches!(self, ServiceHealth::Healthy | ServiceHealth::Degraded { .. })
    }

    pub fn is_healthy(&self) -> bool {
        matches!(self, ServiceHealth::Healthy)
    }
}

/// Service types that can be monitored
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum ServiceType {
    /// IMAP service for a specific account
    Imap(String),
    /// CalDAV service for calendar operations
    CalDav,
    /// Local database
    Database,
    /// Event bus system
    EventBus,
    /// Notification system
    Notifications,
    /// Search/indexing service
    Search,
}

impl std::fmt::Display for ServiceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServiceType::Imap(account) => write!(f, "IMAP({})", account),
            ServiceType::CalDav => write!(f, "CalDAV"),
            ServiceType::Database => write!(f, "Database"),
            ServiceType::EventBus => write!(f, "EventBus"),
            ServiceType::Notifications => write!(f, "Notifications"),
            ServiceType::Search => write!(f, "Search"),
        }
    }
}

/// Health check result
#[derive(Debug, Clone)]
pub struct HealthCheckResult {
    pub service: ServiceType,
    pub health: ServiceHealth,
    pub response_time: Duration,
    pub timestamp: Instant,
    pub details: Option<String>,
}

/// Service health monitor
pub struct ServiceHealthMonitor {
    health_status: Arc<RwLock<HashMap<ServiceType, HealthCheckResult>>>,
    check_interval: Duration,
    #[allow(dead_code)]
    unhealthy_threshold: Duration,
    #[allow(dead_code)]
    recovery_threshold: Duration,
}

impl ServiceHealthMonitor {
    pub fn new() -> Self {
        Self {
            health_status: Arc::new(RwLock::new(HashMap::new())),
            check_interval: Duration::from_secs(30),
            unhealthy_threshold: Duration::from_secs(10),
            recovery_threshold: Duration::from_secs(5),
        }
    }

    pub fn with_config(
        check_interval: Duration,
        unhealthy_threshold: Duration,
        recovery_threshold: Duration,
    ) -> Self {
        Self {
            health_status: Arc::new(RwLock::new(HashMap::new())),
            check_interval,
            unhealthy_threshold,
            recovery_threshold,
        }
    }

    /// Start health monitoring for all services
    pub async fn start_monitoring(&self) {
        let health_status = Arc::clone(&self.health_status);
        let check_interval = self.check_interval;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(check_interval);
            
            loop {
                interval.tick().await;
                
                // Perform health checks for all registered services
                let services = {
                    let status_map = health_status.read().await;
                    status_map.keys().cloned().collect::<Vec<_>>()
                };

                for service in services {
                    if let Err(e) = Self::perform_health_check(&health_status, &service).await {
                        error!("Health check failed for {}: {}", service, e);
                    }
                }
            }
        });
    }

    /// Register a service for health monitoring
    pub async fn register_service(&self, service: ServiceType) {
        let mut status_map = self.health_status.write().await;
        status_map.insert(service.clone(), HealthCheckResult {
            service: service.clone(),
            health: ServiceHealth::Unknown,
            response_time: Duration::from_millis(0),
            timestamp: Instant::now(),
            details: None,
        });
        
        debug!("Registered service for health monitoring: {}", service);
    }

    /// Update service health status
    pub async fn update_service_health(
        &self,
        service: ServiceType,
        health: ServiceHealth,
        response_time: Duration,
        details: Option<String>,
    ) {
        let mut status_map = self.health_status.write().await;
        
        let result = HealthCheckResult {
            service: service.clone(),
            health: health.clone(),
            response_time,
            timestamp: Instant::now(),
            details,
        };

        // Log health status changes
        if let Some(previous) = status_map.get(&service) {
            if previous.health != health {
                match &health {
                    ServiceHealth::Healthy => info!("Service {} recovered", service),
                    ServiceHealth::Degraded { reason } => warn!("Service {} degraded: {}", service, reason),
                    ServiceHealth::Unhealthy { reason, .. } => error!("Service {} became unhealthy: {}", service, reason),
                    ServiceHealth::Unknown => debug!("Service {} status unknown", service),
                }
            }
        }

        status_map.insert(service, result);
    }

    /// Get current health status of a service
    pub async fn get_service_health(&self, service: &ServiceType) -> Option<ServiceHealth> {
        let status_map = self.health_status.read().await;
        status_map.get(service).map(|result| result.health.clone())
    }

    /// Get health status of all services
    pub async fn get_all_health_status(&self) -> HashMap<ServiceType, HealthCheckResult> {
        let status_map = self.health_status.read().await;
        status_map.clone()
    }

    /// Check if a service is available (healthy or degraded)
    pub async fn is_service_available(&self, service: &ServiceType) -> bool {
        if let Some(health) = self.get_service_health(service).await {
            health.is_available()
        } else {
            false
        }
    }

    /// Get unhealthy services
    pub async fn get_unhealthy_services(&self) -> Vec<ServiceType> {
        let status_map = self.health_status.read().await;
        status_map.values()
            .filter(|result| matches!(result.health, ServiceHealth::Unhealthy { .. }))
            .map(|result| result.service.clone())
            .collect()
    }

    /// Perform health check for a specific service
    async fn perform_health_check(
        health_status: &Arc<RwLock<HashMap<ServiceType, HealthCheckResult>>>,
        service: &ServiceType,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let start_time = Instant::now();
        
        // Perform service-specific health check
        let (health, details) = match service {
            ServiceType::Database => {
                // Simulate database health check
                // In a real implementation, this would ping the database
                (ServiceHealth::Healthy, None)
            }
            ServiceType::EventBus => {
                // Check event bus health
                (ServiceHealth::Healthy, Some("Event bus operational".to_string()))
            }
            ServiceType::Notifications => {
                // Check notification system
                (ServiceHealth::Healthy, None)
            }
            ServiceType::Search => {
                // Check search service
                (ServiceHealth::Healthy, None)
            }
            ServiceType::Imap(_account_id) => {
                // IMAP health check would be performed by trying a simple command
                (ServiceHealth::Healthy, None)
            }
            ServiceType::CalDav => {
                // CalDAV health check
                (ServiceHealth::Healthy, None)
            }
        };

        let response_time = start_time.elapsed();
        
        // Update health status
        {
            let mut status_map = health_status.write().await;
            status_map.insert(service.clone(), HealthCheckResult {
                service: service.clone(),
                health,
                response_time,
                timestamp: Instant::now(),
                details,
            });
        }

        Ok(())
    }
}

/// Graceful degradation strategies
pub struct GracefulDegradation {
    health_monitor: Arc<ServiceHealthMonitor>,
}

impl GracefulDegradation {
    pub fn new(health_monitor: Arc<ServiceHealthMonitor>) -> Self {
        Self { health_monitor }
    }

    /// Determine if email operations should proceed based on service health
    pub async fn should_attempt_email_operation(&self, account_id: &str) -> DegradationDecision {
        let imap_service = ServiceType::Imap(account_id.to_string());
        let database_service = ServiceType::Database;

        let imap_health = self.health_monitor.get_service_health(&imap_service).await;
        let db_health = self.health_monitor.get_service_health(&database_service).await;

        match (imap_health, db_health) {
            (Some(ServiceHealth::Healthy), Some(ServiceHealth::Healthy)) => {
                DegradationDecision::Proceed
            }
            (Some(ServiceHealth::Degraded { .. }), Some(ServiceHealth::Healthy)) => {
                DegradationDecision::ProceedWithCaution { 
                    reason: "IMAP service is degraded".to_string(),
                    fallback_strategy: Some("Use cached data where possible".to_string()),
                }
            }
            (Some(ServiceHealth::Healthy), Some(ServiceHealth::Degraded { .. })) => {
                DegradationDecision::ProceedWithCaution {
                    reason: "Database is degraded".to_string(), 
                    fallback_strategy: Some("Cache writes locally".to_string()),
                }
            }
            (Some(ServiceHealth::Unhealthy { .. }), _) => {
                DegradationDecision::Fallback {
                    reason: "IMAP service is unhealthy".to_string(),
                    strategy: FallbackStrategy::CacheOnly,
                }
            }
            (_, Some(ServiceHealth::Unhealthy { .. })) => {
                DegradationDecision::Fallback {
                    reason: "Database is unhealthy".to_string(),
                    strategy: FallbackStrategy::ReadOnly,
                }
            }
            _ => {
                DegradationDecision::Fallback {
                    reason: "Service status unknown".to_string(),
                    strategy: FallbackStrategy::Minimal,
                }
            }
        }
    }

    /// Determine if calendar operations should proceed
    pub async fn should_attempt_calendar_operation(&self) -> DegradationDecision {
        let caldav_service = ServiceType::CalDav;
        let database_service = ServiceType::Database;

        let caldav_health = self.health_monitor.get_service_health(&caldav_service).await;
        let db_health = self.health_monitor.get_service_health(&database_service).await;

        match (caldav_health, db_health) {
            (Some(ServiceHealth::Healthy), Some(ServiceHealth::Healthy)) => {
                DegradationDecision::Proceed
            }
            (Some(ServiceHealth::Degraded { .. }), Some(ServiceHealth::Healthy)) => {
                DegradationDecision::ProceedWithCaution {
                    reason: "CalDAV service is degraded".to_string(),
                    fallback_strategy: Some("Show cached calendar data".to_string()),
                }
            }
            (Some(ServiceHealth::Unhealthy { .. }), _) => {
                DegradationDecision::Fallback {
                    reason: "CalDAV service is unhealthy".to_string(),
                    strategy: FallbackStrategy::CacheOnly,
                }
            }
            (_, Some(ServiceHealth::Unhealthy { .. })) => {
                DegradationDecision::Fallback {
                    reason: "Database is unhealthy".to_string(),
                    strategy: FallbackStrategy::ReadOnly,
                }
            }
            _ => {
                DegradationDecision::Fallback {
                    reason: "Service status unknown".to_string(),
                    strategy: FallbackStrategy::CacheOnly,
                }
            }
        }
    }

    /// Check if search operations should be attempted
    pub async fn should_attempt_search(&self) -> DegradationDecision {
        let search_service = ServiceType::Search;
        let database_service = ServiceType::Database;

        let search_health = self.health_monitor.get_service_health(&search_service).await;
        let db_health = self.health_monitor.get_service_health(&database_service).await;

        match (search_health, db_health) {
            (Some(ServiceHealth::Healthy), Some(ServiceHealth::Healthy)) => {
                DegradationDecision::Proceed
            }
            (Some(ServiceHealth::Degraded { .. }), Some(ServiceHealth::Healthy)) => {
                DegradationDecision::ProceedWithCaution {
                    reason: "Search service is degraded".to_string(),
                    fallback_strategy: Some("Basic text search only".to_string()),
                }
            }
            (Some(ServiceHealth::Unhealthy { .. }), _) => {
                DegradationDecision::Fallback {
                    reason: "Search service is unhealthy".to_string(),
                    strategy: FallbackStrategy::BasicSearch,
                }
            }
            (_, Some(ServiceHealth::Unhealthy { .. })) => {
                DegradationDecision::Fallback {
                    reason: "Database is unhealthy".to_string(),
                    strategy: FallbackStrategy::Minimal,
                }
            }
            _ => {
                DegradationDecision::ProceedWithCaution {
                    reason: "Service status unknown".to_string(),
                    fallback_strategy: Some("Use simple search".to_string()),
                }
            }
        }
    }
}

/// Decision about how to proceed when services might be degraded
#[derive(Debug, Clone)]
pub enum DegradationDecision {
    /// Proceed with normal operation
    Proceed,
    /// Proceed but with caution and potential fallbacks
    ProceedWithCaution {
        reason: String,
        fallback_strategy: Option<String>,
    },
    /// Use fallback strategy instead of normal operation
    Fallback {
        reason: String,
        strategy: FallbackStrategy,
    },
}

/// Available fallback strategies
#[derive(Debug, Clone)]
pub enum FallbackStrategy {
    /// Only use cached/local data, no remote operations
    CacheOnly,
    /// Read-only mode, disable writes
    ReadOnly,
    /// Minimal functionality only
    Minimal,
    /// Basic search without advanced features
    BasicSearch,
}

impl DegradationDecision {
    pub fn should_proceed(&self) -> bool {
        matches!(self, DegradationDecision::Proceed | DegradationDecision::ProceedWithCaution { .. })
    }

    pub fn get_user_message(&self) -> Option<String> {
        match self {
            DegradationDecision::Proceed => None,
            DegradationDecision::ProceedWithCaution { reason, fallback_strategy } => {
                let mut msg = format!("⚠️ {}", reason);
                if let Some(strategy) = fallback_strategy {
                    msg.push_str(&format!(" ({})", strategy));
                }
                Some(msg)
            }
            DegradationDecision::Fallback { reason, strategy } => {
                Some(format!("⚠️ {}: Using {} mode", reason, match strategy {
                    FallbackStrategy::CacheOnly => "offline",
                    FallbackStrategy::ReadOnly => "read-only",
                    FallbackStrategy::Minimal => "minimal",
                    FallbackStrategy::BasicSearch => "basic search",
                }))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_service_health_monitor_registration() {
        let monitor = ServiceHealthMonitor::new();
        let service = ServiceType::Database;

        monitor.register_service(service.clone()).await;
        
        let health = monitor.get_service_health(&service).await;
        assert!(matches!(health, Some(ServiceHealth::Unknown)));
    }

    #[tokio::test]
    async fn test_health_status_update() {
        let monitor = ServiceHealthMonitor::new();
        let service = ServiceType::Database;

        monitor.register_service(service.clone()).await;
        monitor.update_service_health(
            service.clone(),
            ServiceHealth::Healthy,
            Duration::from_millis(50),
            Some("Database is responsive".to_string()),
        ).await;

        let health = monitor.get_service_health(&service).await;
        assert!(matches!(health, Some(ServiceHealth::Healthy)));
    }

    #[tokio::test]
    async fn test_graceful_degradation_decisions() {
        let monitor = Arc::new(ServiceHealthMonitor::new());
        let degradation = GracefulDegradation::new(Arc::clone(&monitor));

        // Register services
        monitor.register_service(ServiceType::Database).await;
        monitor.register_service(ServiceType::Imap("test@example.com".to_string())).await;

        // Test with healthy services
        monitor.update_service_health(
            ServiceType::Database,
            ServiceHealth::Healthy,
            Duration::from_millis(10),
            None,
        ).await;

        monitor.update_service_health(
            ServiceType::Imap("test@example.com".to_string()),
            ServiceHealth::Healthy,
            Duration::from_millis(50),
            None,
        ).await;

        let decision = degradation.should_attempt_email_operation("test@example.com").await;
        assert!(matches!(decision, DegradationDecision::Proceed));

        // Test with degraded service
        monitor.update_service_health(
            ServiceType::Imap("test@example.com".to_string()),
            ServiceHealth::Degraded { reason: "Slow response".to_string() },
            Duration::from_millis(2000),
            None,
        ).await;

        let decision = degradation.should_attempt_email_operation("test@example.com").await;
        assert!(matches!(decision, DegradationDecision::ProceedWithCaution { .. }));
    }
}