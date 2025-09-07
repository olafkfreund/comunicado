pub mod simple_client;
pub mod types;
pub mod utils;

// Re-export main types
pub use simple_client::KdeConnectClient;

pub use types::{
    Attachment, ContactInfo, DeviceInfo, MessageType, MobileNotification, NotificationAction,
    SmsConversation, SmsMessage,
};
pub use utils::{format_phone_number, parse_message_timestamp, sanitize_message_content};
