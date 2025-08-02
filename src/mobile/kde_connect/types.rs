use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmsMessage {
    pub id: i32,
    pub body: String,
    pub addresses: Vec<String>,
    pub date: i64,
    pub message_type: MessageType,
    pub read: bool,
    pub thread_id: i64,
    pub sub_id: i64,
    pub attachments: Vec<Attachment>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MessageType {
    Sms = 1,
    Mms = 2,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub part_id: i64,
    pub mime_type: String,
    pub filename: String,
    pub file_size: i64,
    pub data: Option<Vec<u8>>,
    pub download_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmsConversation {
    pub id: i64,
    pub thread_id: i64,
    pub display_name: String,
    pub participants: Vec<ContactInfo>,
    pub message_count: i32,
    pub unread_count: i32,
    pub last_message_date: DateTime<Utc>,
    pub is_archived: bool,
    pub messages: Vec<SmsMessage>, // Full message list when loaded
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactInfo {
    pub address: String,
    pub display_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileNotification {
    pub id: String,
    pub app_name: String,
    pub title: String,
    pub text: String,
    pub icon: Option<String>,
    pub time: i64,
    pub dismissable: bool,
    pub has_reply_action: bool,
    pub reply_id: Option<String>,
    pub actions: Vec<NotificationAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationAction {
    pub key: String,
    pub display_name: String,
}

#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub id: String,
    pub name: String,
    pub is_reachable: bool,
    pub has_sms_plugin: bool,
    pub has_notification_plugin: bool,
    pub device_type: DeviceType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DeviceType {
    Phone,
    Tablet,
    Desktop,
    Unknown,
}

impl SmsMessage {
    pub fn is_outgoing(&self) -> bool {
        // In KDE Connect protocol, outgoing messages can be determined by checking
        // if the sender address matches the local device's phone number or by
        // message direction indicators in the metadata
        // For now, this is a simplified implementation that would need to be
        // enhanced with actual device phone number detection
        false
    }

    pub fn sender(&self) -> Option<&String> {
        self.addresses.first()
    }

    pub fn formatted_date(&self) -> String {
        let datetime = DateTime::from_timestamp(self.date / 1000, 0) // KDE Connect uses milliseconds
            .unwrap_or_else(|| Utc::now());
        datetime.format("%Y-%m-%d %H:%M").to_string()
    }

    pub fn preview_text(&self, max_length: usize) -> String {
        if self.body.len() <= max_length {
            self.body.clone()
        } else {
            format!("{}...", &self.body[..max_length])
        }
    }

    pub fn is_mms(&self) -> bool {
        matches!(self.message_type, MessageType::Mms) || !self.attachments.is_empty()
    }

    pub fn has_attachments(&self) -> bool {
        !self.attachments.is_empty()
    }

    pub fn attachment_count(&self) -> usize {
        self.attachments.len()
    }

    pub fn word_count(&self) -> usize {
        self.body.split_whitespace().count()
    }

    pub fn character_count(&self) -> usize {
        self.body.chars().count()
    }
}

impl SmsConversation {
    pub fn has_unread(&self) -> bool {
        self.unread_count > 0
    }

    pub fn last_activity_formatted(&self) -> String {
        self.last_message_date.format("%Y-%m-%d %H:%M").to_string()
    }

    pub fn primary_address(&self) -> Option<&ContactInfo> {
        self.participants.first()
    }

    pub fn is_group_conversation(&self) -> bool {
        self.participants.len() > 1
    }

    pub fn participant_names(&self) -> Vec<String> {
        self.participants
            .iter()
            .map(|contact| {
                contact.display_name
                    .clone()
                    .unwrap_or_else(|| contact.address.clone())
            })
            .collect()
    }

    pub fn update_last_activity(&mut self, timestamp: DateTime<Utc>) {
        self.last_message_date = timestamp;
    }

    pub fn increment_message_count(&mut self) {
        self.message_count += 1;
    }

    pub fn increment_unread_count(&mut self) {
        self.unread_count += 1;
    }

    pub fn mark_all_read(&mut self) {
        self.unread_count = 0;
    }
}

impl ContactInfo {
    pub fn new(address: String, display_name: Option<String>) -> Self {
        Self {
            address,
            display_name,
        }
    }

    pub fn display_text(&self) -> &str {
        self.display_name.as_ref().unwrap_or(&self.address)
    }

    pub fn from_address(address: &str) -> Self {
        Self::new(address.to_string(), None)
    }

    pub fn is_phone_number(&self) -> bool {
        self.address.starts_with('+') || 
        self.address.chars().all(|c| c.is_ascii_digit() || c == '-' || c == ' ' || c == '(' || c == ')')
    }
}

impl MobileNotification {
    pub fn is_sms_notification(&self) -> bool {
        matches!(self.app_name.as_str(), "Messages" | "SMS" | "Messaging")
    }

    pub fn priority_score(&self) -> u8 {
        match self.app_name.as_str() {
            "Messages" | "SMS" | "WhatsApp" | "Telegram" | "Signal" => 5,
            "Email" | "Gmail" | "Outlook" | "Mail" => 3,
            "Phone" | "Calls" => 4,
            "Calendar" | "Events" => 2,
            _ => 1,
        }
    }

    pub fn should_show_preview(&self) -> bool {
        // Don't show preview for sensitive apps or during quiet hours
        !matches!(self.app_name.as_str(), "Banking" | "Wallet" | "Password")
    }

    pub fn formatted_time(&self) -> String {
        let datetime = DateTime::from_timestamp(self.time / 1000, 0)
            .unwrap_or_else(|| Utc::now());
        datetime.format("%H:%M").to_string()
    }

    pub fn age_minutes(&self) -> i64 {
        let now = Utc::now().timestamp() * 1000; // Convert to milliseconds
        (now - self.time) / (60 * 1000) // Convert back to minutes
    }

    pub fn is_recent(&self) -> bool {
        self.age_minutes() < 30 // Less than 30 minutes old
    }
}

impl DeviceInfo {
    pub fn new(id: String, name: String) -> Self {
        Self {
            id,
            name,
            is_reachable: false,
            has_sms_plugin: false,
            has_notification_plugin: false,
            device_type: DeviceType::Unknown,
        }
    }

    pub fn can_send_sms(&self) -> bool {
        self.is_reachable && self.has_sms_plugin
    }

    pub fn can_forward_notifications(&self) -> bool {
        self.is_reachable && self.has_notification_plugin
    }

    pub fn capabilities(&self) -> Vec<&'static str> {
        let mut caps = Vec::new();
        
        if self.has_sms_plugin {
            caps.push("SMS");
        }
        if self.has_notification_plugin {
            caps.push("Notifications");
        }
        if !self.is_reachable {
            caps.push("Offline");
        }
        
        caps
    }

    pub fn device_type_icon(&self) -> &'static str {
        match self.device_type {
            DeviceType::Phone => "📱",
            DeviceType::Tablet => "📱", // Could use different icon
            DeviceType::Desktop => "💻",
            DeviceType::Unknown => "📟",
        }
    }
}

impl Attachment {
    pub fn new(part_id: i64, mime_type: String, filename: String, data: Vec<u8>) -> Self {
        let file_size = data.len() as i64;
        Self {
            part_id,
            mime_type,
            filename,
            file_size,
            data: Some(data),
            download_url: None,
        }
    }

    pub fn with_download_url(part_id: i64, mime_type: String, filename: String, download_url: String, file_size: i64) -> Self {
        Self {
            part_id,
            mime_type,
            filename,
            file_size,
            data: None,
            download_url: Some(download_url),
        }
    }

    pub fn is_image(&self) -> bool {
        self.mime_type.starts_with("image/")
    }

    pub fn is_video(&self) -> bool {
        self.mime_type.starts_with("video/")
    }

    pub fn is_audio(&self) -> bool {
        self.mime_type.starts_with("audio/")
    }

    pub fn file_extension(&self) -> Option<&str> {
        if !self.filename.is_empty() {
            self.filename.split('.').last()
        } else {
            // Infer from MIME type
            match self.mime_type.as_str() {
                "image/jpeg" => Some("jpg"),
                "image/png" => Some("png"),
                "image/gif" => Some("gif"),
                "video/mp4" => Some("mp4"),
                "audio/mpeg" => Some("mp3"),
                _ => None,
            }
        }
    }

    pub fn size_bytes(&self) -> i64 {
        self.file_size
    }

    pub fn size_formatted(&self) -> String {
        let size = self.size_bytes() as f64;
        
        if size < 1024.0 {
            format!("{} B", size)
        } else if size < 1024.0 * 1024.0 {
            format!("{:.1} KB", size / 1024.0)
        } else {
            format!("{:.1} MB", size / (1024.0 * 1024.0))
        }
    }
}

impl NotificationAction {
    pub fn new(key: String, display_name: String) -> Self {
        Self { key, display_name }
    }

    pub fn is_reply_action(&self) -> bool {
        self.key == "reply" || self.display_name.to_lowercase().contains("reply")
    }

    pub fn is_dismiss_action(&self) -> bool {
        self.key == "dismiss" || self.display_name.to_lowercase().contains("dismiss")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sms_message() {
        let message = SmsMessage {
            id: 123,
            body: "Test message".to_string(),
            addresses: vec!["+1234567890".to_string()],
            date: chrono::Utc::now().timestamp() * 1000,
            message_type: MessageType::Sms,
            read: false,
            thread_id: 1,
            sub_id: 1,
            attachments: vec![],
        };

        assert_eq!(message.preview_text(10), "Test messa...");
        assert_eq!(message.sender(), Some(&"+1234567890".to_string()));
        assert!(!message.is_outgoing());
        assert!(!message.is_mms());
        assert!(!message.has_attachments());
        assert_eq!(message.word_count(), 2);
        assert_eq!(message.character_count(), 12);
    }

    #[test]
    fn test_sms_conversation() {
        let mut conversation = SmsConversation {
            id: 1,
            thread_id: 1,
            display_name: "John Doe".to_string(),
            participants: vec![ContactInfo::new("+1234567890".to_string(), Some("John Doe".to_string()))],
            message_count: 5,
            unread_count: 2,
            last_message_date: chrono::Utc::now(),
            is_archived: false,
            messages: vec![],
        };

        assert!(conversation.has_unread());
        assert!(!conversation.is_group_conversation());
        assert_eq!(conversation.participant_names(), vec!["John Doe".to_string()]);

        conversation.mark_all_read();
        assert!(!conversation.has_unread());
        assert_eq!(conversation.unread_count, 0);
    }

    #[test]
    fn test_contact_info() {
        let contact = ContactInfo::new("+1234567890".to_string(), Some("John Doe".to_string()));
        assert_eq!(contact.display_text(), "John Doe");
        assert!(contact.is_phone_number());

        let email_contact = ContactInfo::new("john@example.com".to_string(), None);
        assert_eq!(email_contact.display_text(), "john@example.com");
        assert!(!email_contact.is_phone_number());
    }

    #[test]
    fn test_mobile_notification() {
        let notification = MobileNotification {
            id: "test".to_string(),
            app_name: "Messages".to_string(),
            title: "New message".to_string(),
            text: "Hello world".to_string(),
            icon: None,
            time: chrono::Utc::now().timestamp() * 1000,
            dismissable: true,
            has_reply_action: true,
            reply_id: Some("reply-123".to_string()),
            actions: vec![],
        };

        assert!(notification.is_sms_notification());
        assert_eq!(notification.priority_score(), 5);
        assert!(notification.should_show_preview());
        assert!(notification.is_recent());
    }

    #[test]
    fn test_device_info() {
        let mut device = DeviceInfo::new("device-123".to_string(), "My Phone".to_string());
        assert!(!device.can_send_sms());
        assert!(!device.can_forward_notifications());

        device.is_reachable = true;
        device.has_sms_plugin = true;
        device.has_notification_plugin = true;
        device.device_type = DeviceType::Phone;

        assert!(device.can_send_sms());
        assert!(device.can_forward_notifications());
        assert_eq!(device.device_type_icon(), "📱");
        assert_eq!(device.capabilities(), vec!["SMS", "Notifications"]);
    }

    #[test]
    fn test_attachment() {
        let attachment = Attachment::new(
            1,
            "image/jpeg".to_string(),
            "photo.jpg".to_string(),
            vec![1, 2, 3, 4, 5], // 5 bytes
        );

        assert!(attachment.is_image());
        assert!(!attachment.is_video());
        assert!(!attachment.is_audio());
        assert_eq!(attachment.file_extension(), Some("jpg"));
        assert_eq!(attachment.size_bytes(), 5);
        assert_eq!(attachment.size_formatted(), "5 B");
    }

    #[test]
    fn test_notification_action() {
        let reply_action = NotificationAction::new("reply".to_string(), "Reply".to_string());
        assert!(reply_action.is_reply_action());
        assert!(!reply_action.is_dismiss_action());

        let dismiss_action = NotificationAction::new("dismiss".to_string(), "Dismiss".to_string());
        assert!(!dismiss_action.is_reply_action());
        assert!(dismiss_action.is_dismiss_action());
    }
}