use crate::email::EmailNotification;
use crate::calendar::CalendarNotification;
use std::process::Command;
use tokio::sync::broadcast;

/// Desktop notification service for email events
pub struct DesktopNotificationService {
    enabled: bool,
    show_preview: bool,
}

impl DesktopNotificationService {
    /// Create a new desktop notification service
    pub fn new() -> Self {
        Self {
            enabled: true,
            show_preview: true,
        }
    }
    
    /// Create a disabled notification service
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            show_preview: false,
        }
    }
    
    /// Enable or disable notifications
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
    
    /// Enable or disable message previews in notifications
    pub fn set_preview_enabled(&mut self, enabled: bool) {
        self.show_preview = enabled;
    }
    
    /// Start listening for email notifications and display desktop notifications
    pub async fn start(&self, mut receiver: broadcast::Receiver<EmailNotification>) {
        if !self.enabled {
            tracing::info!("Desktop notifications are disabled");
            return;
        }
        
        let show_preview = self.show_preview;
        
        tokio::spawn(async move {
            while let Ok(notification) = receiver.recv().await {
                match notification {
                    EmailNotification::NewMessage { account_id, folder_name, message } => {
                        let title = format!("New Email - {}", account_id);
                        let body = if show_preview {
                            let from_display = if let Some(ref name) = message.from_name {
                                format!("{} <{}>", name, message.from_addr)
                            } else {
                                message.from_addr.clone()
                            };
                            format!("From: {}\nSubject: {}\nFolder: {}", 
                                   from_display,
                                   if message.subject.is_empty() { "(No Subject)" } else { &message.subject },
                                   folder_name)
                        } else {
                            format!("New message in folder: {}", folder_name)
                        };
                        
                        Self::send_notification(&title, &body, "mail-message-new").await;
                    }
                    EmailNotification::SyncCompleted { account_id, folder_name, new_count, .. } => {
                        if new_count > 0 {
                            let title = format!("Email Sync - {}", account_id);
                            let body = format!("Received {} new message{} in {}", 
                                             new_count, 
                                             if new_count == 1 { "" } else { "s" },
                                             folder_name);
                            
                            Self::send_notification(&title, &body, "mail-folder-inbox").await;
                        }
                    }
                    EmailNotification::SyncFailed { account_id, folder_name, error } => {
                        let title = format!("Sync Failed - {}", account_id);
                        let body = format!("Failed to sync folder {}: {}", folder_name, error);
                        
                        Self::send_notification(&title, &body, "dialog-error").await;
                    }
                    _ => {
                        // Other notification types don't trigger desktop notifications
                    }
                }
            }
        });
    }
    
    /// Start listening for calendar notifications and display desktop notifications
    pub async fn start_calendar(&self, mut receiver: broadcast::Receiver<CalendarNotification>) {
        if !self.enabled {
            tracing::info!("Desktop calendar notifications are disabled");
            return;
        }
        
        let show_preview = self.show_preview;
        
        tokio::spawn(async move {
            while let Ok(notification) = receiver.recv().await {
                match notification {
                    CalendarNotification::EventCreated { calendar_id, event } => {
                        let title = format!("Event Created - {}", calendar_id);
                        let body = if show_preview {
                            format!("Event: {}\nDate: {}", 
                                   event.title,
                                   event.start_time.format("%Y-%m-%d %H:%M"))
                        } else {
                            "New calendar event created".to_string()
                        };
                        
                        Self::send_notification(&title, &body, "calendar").await;
                    }
                    CalendarNotification::EventReminder { calendar_id: _, event, minutes_until } => {
                        let title = "Calendar Reminder";
                        let body = if show_preview {
                            format!("Upcoming: {}\nStarts in {} minute{}", 
                                   event.title,
                                   minutes_until,
                                   if minutes_until == 1 { "" } else { "s" })
                        } else {
                            format!("Event starting in {} minutes", minutes_until)
                        };
                        
                        Self::send_notification(title, &body, "appointment-soon").await;
                    }
                    CalendarNotification::SyncCompleted { calendar_id, new_count, updated_count } => {
                        if new_count > 0 || updated_count > 0 {
                            let title = format!("Calendar Sync - {}", calendar_id);
                            let body = format!("Sync complete: {} new, {} updated events", new_count, updated_count);
                            
                            Self::send_notification(&title, &body, "calendar").await;
                        }
                    }
                    CalendarNotification::SyncFailed { calendar_id, error } => {
                        let title = format!("Calendar Sync Failed - {}", calendar_id);
                        let body = format!("Sync failed: {}", error);
                        
                        Self::send_notification(&title, &body, "dialog-error").await;
                    }
                    CalendarNotification::RSVPSent { event_id: _, response } => {
                        let title = "RSVP Response Sent";
                        let body = format!("Response: {}", response);
                        
                        Self::send_notification(title, &body, "mail-send").await;
                    }
                    _ => {
                        // Other notification types don't trigger desktop notifications
                    }
                }
            }
        });
    }
    
    /// Send a desktop notification using the system's notification daemon
    async fn send_notification(title: &str, body: &str, icon: &str) {
        // Try different notification methods in order of preference
        
        // First try notify-send (most common on Linux)
        if let Ok(_) = Command::new("notify-send")
            .arg("--app-name=Comunicado")
            .arg(format!("--icon={}", icon))
            .arg("--urgency=normal")
            .arg("--expire-time=5000") // 5 seconds
            .arg(title)
            .arg(body)
            .output()
        {
            tracing::debug!("Desktop notification sent via notify-send: {}", title);
            return;
        }
        
        // Try osascript for macOS
        if cfg!(target_os = "macos") {
            let script = format!(
                r#"display notification "{}" with title "{}" subtitle "Comunicado""#,
                body.replace('"', r#"\""#),
                title.replace('"', r#"\""#)
            );
            
            if let Ok(_) = Command::new("osascript")
                .arg("-e")
                .arg(&script)
                .output()
            {
                tracing::debug!("Desktop notification sent via osascript: {}", title);
                return;
            }
        }
        
        // Try terminal-notifier for macOS (alternative)
        if cfg!(target_os = "macos") {
            if let Ok(_) = Command::new("terminal-notifier")
                .arg("-title")
                .arg("Comunicado")
                .arg("-subtitle")
                .arg(title)
                .arg("-message")
                .arg(body)
                .arg("-sound")
                .arg("default")
                .output()
            {
                tracing::debug!("Desktop notification sent via terminal-notifier: {}", title);
                return;
            }
        }
        
        // Fallback: log the notification
        tracing::info!("Desktop notification (fallback): {} - {}", title, body);
    }
    
    /// Check if desktop notifications are supported on this system
    pub fn is_supported() -> bool {
        // Check for notify-send on Linux
        if Command::new("which").arg("notify-send").output().is_ok() {
            if let Ok(output) = Command::new("which").arg("notify-send").output() {
                if output.status.success() {
                    return true;
                }
            }
        }
        
        // Check for osascript on macOS
        if cfg!(target_os = "macos") {
            if let Ok(output) = Command::new("which").arg("osascript").output() {
                if output.status.success() {
                    return true;
                }
            }
        }
        
        // Check for terminal-notifier on macOS
        if cfg!(target_os = "macos") {
            if let Ok(output) = Command::new("which").arg("terminal-notifier").output() {
                if output.status.success() {
                    return true;
                }
            }
        }
        
        false
    }
    
    /// Send a test notification
    pub async fn send_test_notification() {
        Self::send_notification(
            "Comunicado Test",
            "Desktop notifications are working correctly!",
            "mail-message-new"
        ).await;
    }
}

impl Default for DesktopNotificationService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_notification_service_creation() {
        let service = DesktopNotificationService::new();
        assert!(service.enabled);
        assert!(service.show_preview);
        
        let disabled_service = DesktopNotificationService::disabled();
        assert!(!disabled_service.enabled);
        assert!(!disabled_service.show_preview);
    }
    
    #[test]
    fn test_enable_disable() {
        let mut service = DesktopNotificationService::new();
        
        service.set_enabled(false);
        assert!(!service.enabled);
        
        service.set_preview_enabled(false);
        assert!(!service.show_preview);
    }
    
    #[tokio::test]
    async fn test_notification_support_check() {
        // This test will vary by platform, but should not panic
        let _supported = DesktopNotificationService::is_supported();
    }
}