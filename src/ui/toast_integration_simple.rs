/// Simple toast notification integration for UI
/// 
/// This provides a simple approach to integrate toast notifications with the existing UI architecture
/// by working directly with the UI's toast manager instead of using a separate service.

use crate::email::EmailNotification;
use crate::calendar::CalendarNotification;
use crate::tea::message::ToastLevel;
use crate::ui::toast::ToastManager;
use crate::performance::background_processor::{TaskResult, TaskStatus, TaskResultData};
use tokio::time::Duration;
use tracing::debug;

/// Simple toast integration that works directly with the UI's toast manager
pub struct SimpleToastIntegration;

impl SimpleToastIntegration {
    /// Handle email notification and add toast to manager
    pub fn handle_email_notification(toast_manager: &mut ToastManager, notification: EmailNotification) {
        match notification {
            EmailNotification::NewMessage { account_id, folder_name, message } => {
                let sender = message.from_name.as_deref().unwrap_or("Unknown");
                let subject = if message.subject.is_empty() { "(No subject)" } else { &message.subject };
                
                let display_subject = if subject.len() > 40 {
                    format!("{}...", &subject[..37])
                } else {
                    subject.to_string()
                };

                let toast_message = if folder_name == "INBOX" {
                    format!("📧 {} • {}", sender, display_subject)
                } else {
                    format!("📧 {} • {} • {}", folder_name, sender, display_subject)
                };

                toast_manager.show_with_duration(toast_message, ToastLevel::Info, Duration::from_secs(4));
                debug!("Showed new email toast for account: {}, folder: {}", account_id, folder_name);
            }

            EmailNotification::MessageUpdated { account_id, folder_name, .. } => {
                let toast_message = format!("📝 Email updated in {}", folder_name);
                toast_manager.quick(toast_message, ToastLevel::Info);
                debug!("Showed email update toast for account: {}, folder: {}", account_id, folder_name);
            }

            EmailNotification::MessageDeleted { account_id, folder_name, .. } => {
                let toast_message = format!("🗑️ Email deleted from {}", folder_name);
                toast_manager.quick(toast_message, ToastLevel::Info);
                debug!("Showed email deletion toast for account: {}, folder: {}", account_id, folder_name);
            }

            EmailNotification::SyncStarted { account_id, folder_name } => {
                let toast_message = format!("🔄 Syncing {}", folder_name);
                toast_manager.quick(toast_message, ToastLevel::Info);
                debug!("Showed sync started toast for account: {}, folder: {}", account_id, folder_name);
            }

            EmailNotification::SyncCompleted { account_id, folder_name, new_count, updated_count } => {
                if new_count > 0 || updated_count > 0 {
                    let toast_message = if new_count > 0 && updated_count > 0 {
                        format!("✅ {} synced • {} new, {} updated", folder_name, new_count, updated_count)
                    } else if new_count > 0 {
                        format!("✅ {} synced • {} new", folder_name, new_count)
                    } else {
                        format!("✅ {} synced • {} updated", folder_name, updated_count)
                    };
                    toast_manager.success(toast_message);
                } else {
                    toast_manager.quick(format!("✅ {} up to date", folder_name), ToastLevel::Success);
                }
                debug!("Showed sync completion toast for account: {}, folder: {}", account_id, folder_name);
            }

            EmailNotification::SyncFailed { account_id, folder_name, error } => {
                let toast_message = format!("❌ Failed to sync {} • {}", folder_name, 
                    if error.len() > 30 { format!("{}...", &error[..27]) } else { error });
                toast_manager.show_with_duration(toast_message, ToastLevel::Error, Duration::from_secs(6));
                debug!("Showed sync failure toast for account: {}, folder: {}", account_id, folder_name);
            }
        }
    }

    /// Handle calendar notification and add toast to manager
    pub fn handle_calendar_notification(toast_manager: &mut ToastManager, notification: CalendarNotification) {
        match notification {
            CalendarNotification::EventCreated { calendar_id, event } => {
                let title = if event.title.is_empty() { "Untitled Event" } else { &event.title };
                let display_title = if title.len() > 30 {
                    format!("{}...", &title[..27])
                } else {
                    title.to_string()
                };
                
                let toast_message = format!("📅 Event created • {}", display_title);
                toast_manager.success(toast_message);
                debug!("Showed event creation toast for calendar: {}", calendar_id);
            }

            CalendarNotification::EventUpdated { calendar_id, event } => {
                let title = if event.title.is_empty() { "Untitled Event" } else { &event.title };
                let display_title = if title.len() > 30 {
                    format!("{}...", &title[..27])
                } else {
                    title.to_string()
                };
                
                let toast_message = format!("📝 Event updated • {}", display_title);
                toast_manager.info(toast_message);
                debug!("Showed event update toast for calendar: {}", calendar_id);
            }

            CalendarNotification::EventDeleted { calendar_id, event_id: _ } => {
                let toast_message = "🗑️ Event deleted".to_string();
                toast_manager.info(toast_message);
                debug!("Showed event deletion toast for calendar: {}", calendar_id);
            }

            CalendarNotification::EventReminder { calendar_id: _, event, minutes_until } => {
                let title = if event.title.is_empty() { "Untitled Event" } else { &event.title };
                let display_title = if title.len() > 25 {
                    format!("{}...", &title[..22])
                } else {
                    title.to_string()
                };

                let toast_message = if minutes_until == 0 {
                    format!("⏰ Starting now • {}", display_title)
                } else if minutes_until < 60 {
                    format!("⏰ In {} min • {}", minutes_until, display_title)
                } else {
                    let hours = minutes_until / 60;
                    let mins = minutes_until % 60;
                    if mins == 0 {
                        format!("⏰ In {}h • {}", hours, display_title)
                    } else {
                        format!("⏰ In {}h{}m • {}", hours, mins, display_title)
                    }
                };

                toast_manager.show_with_duration(toast_message, ToastLevel::Warning, Duration::from_secs(8));
                debug!("Showed reminder toast for event, {} minutes until start", minutes_until);
            }

            CalendarNotification::SyncCompleted { calendar_id, new_count, updated_count } => {
                if new_count > 0 || updated_count > 0 {
                    let toast_message = if new_count > 0 && updated_count > 0 {
                        format!("✅ Calendar synced • {} new, {} updated", new_count, updated_count)
                    } else if new_count > 0 {
                        format!("✅ Calendar synced • {} new", new_count)
                    } else {
                        format!("✅ Calendar synced • {} updated", updated_count)
                    };
                    toast_manager.success(toast_message);
                } else {
                    toast_manager.quick("✅ Calendar up to date".to_string(), ToastLevel::Success);
                }
                debug!("Showed calendar sync completion toast for calendar: {}", calendar_id);
            }

            CalendarNotification::SyncStarted { calendar_id } => {
                let toast_message = "🔄 Syncing calendar".to_string();
                toast_manager.quick(toast_message, ToastLevel::Info);
                debug!("Showed calendar sync started toast for calendar: {}", calendar_id);
            }

            CalendarNotification::SyncFailed { calendar_id, error } => {
                let toast_message = format!("❌ Calendar sync failed • {}", 
                    if error.len() > 25 { format!("{}...", &error[..22]) } else { error });
                toast_manager.show_with_duration(toast_message, ToastLevel::Error, Duration::from_secs(5));
                debug!("Showed calendar sync failure toast for calendar: {}", calendar_id);
            }

            CalendarNotification::RSVPSent { event_id: _, response } => {
                let toast_message = format!("📨 RSVP sent • {}", response);
                toast_manager.success(toast_message);
                debug!("Showed RSVP sent toast");
            }
        }
    }

    /// Handle background task completion and add toast to manager
    pub fn handle_task_completion(toast_manager: &mut ToastManager, result: TaskResult) {
        match result.status {
            TaskStatus::Completed => {
                // Only show toast for significant or user-initiated tasks
                if let Some(data) = result.result_data {
                    match data {
                        TaskResultData::SyncProgress(_) => {
                            // Don't show toast for sync progress updates - these are handled by sync completion notifications
                        }
                        TaskResultData::MessageCount(count) if count > 0 => {
                            toast_manager.quick(format!("✅ Background task completed • {} items", count), ToastLevel::Success);
                        }
                        TaskResultData::SearchResults(results) if !results.is_empty() => {
                            toast_manager.success(format!("🔍 Search completed • {} results", results.len()));
                        }
                        TaskResultData::CacheStats(cached) if cached > 100 => {
                            toast_manager.quick(format!("💾 Cache updated • {} items", cached), ToastLevel::Info);
                        }
                        TaskResultData::CalendarSyncResult { events_synced, events_updated, events_deleted } => {
                            if events_synced + events_updated + events_deleted > 0 {
                                toast_manager.success(format!("📅 Calendar synced • {}+{}∆{}-", events_synced, events_updated, events_deleted));
                            }
                        }
                        TaskResultData::CalendarDiscoveryResult { calendars_found } if !calendars_found.is_empty() => {
                            toast_manager.info(format!("🔍 Found {} calendars", calendars_found.len()));
                        }
                        TaskResultData::CalendarDbOperationResult { success: true, affected_rows } if affected_rows > 0 => {
                            toast_manager.quick(format!("💾 Database updated • {} records", affected_rows), ToastLevel::Success);
                        }
                        _ => {
                            // Don't show toast for other completed tasks to avoid spam
                        }
                    }
                }
            }

            TaskStatus::Failed(error) => {
                // Always show failure notifications
                let toast_message = format!("❌ Background task failed • {}", 
                    if error.len() > 30 { format!("{}...", &error[..27]) } else { error });
                toast_manager.show_with_duration(toast_message, ToastLevel::Error, Duration::from_secs(5));
                debug!("Showed task failure toast for task: {}", result.task_id);
            }

            TaskStatus::Cancelled => {
                toast_manager.quick("⏹️ Background task cancelled".to_string(), ToastLevel::Warning);
                debug!("Showed task cancellation toast for task: {}", result.task_id);
            }

            TaskStatus::Queued | TaskStatus::Running => {
                // Don't show toasts for these states
            }
        }
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use uuid::Uuid;
//
//     #[test]
//     fn test_email_notification_handling() {
//         // TODO: Fix this test to use correct StoredMessage structure
//         // Commented out due to field name changes
//     }
// }