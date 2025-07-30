/// Unified notification system for Comunicado
///
/// This module provides a comprehensive notification system that handles both
/// email and calendar notifications through a unified interface. It integrates
/// with the system's native notification service and provides rich notification
/// features including:
///
/// - Desktop notifications with native OS integration
/// - Smart notification batching and filtering
/// - Privacy-aware content display
/// - Calendar reminder management
/// - Cross-platform notification support
/// - Advanced integration with email and calendar systems
/// - Automated reminder scheduling
/// - Notification action handling
pub mod desktop;
pub mod integration;
pub mod manager;
pub mod types;

pub use desktop::DesktopNotificationService;
pub use integration::{NotificationIntegrationService, NotificationStatistics};
pub use manager::UnifiedNotificationManager;
pub use types::{NotificationConfig, NotificationEvent, NotificationPriority};
