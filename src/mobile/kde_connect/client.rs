#[cfg(feature = "kde-connect")]
use dbus::blocking::{Connection, Proxy};
#[cfg(feature = "kde-connect")]
use dbus::{Message, MessageType};

use std::collections::HashMap;
use std::time::Duration;
use anyhow::{Result, Context};
use tokio::sync::mpsc;
use tracing::{info, warn, error, debug};

use super::types::{DeviceInfo, SmsMessage, MobileNotification, MessageType as SmsMessageType, ContactInfo, DeviceType};
use super::utils::{dbus_variant_to_string, dbus_variant_to_i64, dbus_variant_to_bool, parse_message_timestamp};

#[cfg(feature = "kde-connect")]
const KDECONNECT_SERVICE: &str = "org.kde.kdeconnect";
#[cfg(feature = "kde-connect")]
const KDECONNECT_DAEMON_PATH: &str = "/modules/kdeconnect";
#[cfg(feature = "kde-connect")]
const KDECONNECT_DAEMON_INTERFACE: &str = "org.kde.kdeconnect.daemon";

pub struct KdeConnectClient {
    #[cfg(feature = "kde-connect")]
    connection: Connection,
    device_id: Option<String>,
    timeout: Duration,
}

impl KdeConnectClient {
    pub fn new() -> crate::mobile::Result<Self> {
        let connection = Connection::new_session()
            .context("Failed to connect to D-Bus session bus")
            .map_err(|e| crate::mobile::MobileError::DbusConnectionFailed(
                dbus::Error::new_failed(&e.to_string())
            ))?;
        
        Ok(Self {
            connection,
            device_id: None,
            timeout: Duration::from_millis(5000),
        })
    }

    pub fn discover_devices(&self) -> crate::mobile::Result<Vec<DeviceInfo>> {
        debug!("Discovering KDE Connect devices");
        
        let proxy = self.connection.with_proxy(
            KDECONNECT_SERVICE,
            KDECONNECT_DAEMON_PATH,
            self.timeout,
        );

        // Check if KDE Connect daemon is running
        let device_names_result: Result<Vec<String>, dbus::Error> = proxy
            .method_call(KDECONNECT_DAEMON_INTERFACE, "deviceNames", ());

        let device_ids = match device_names_result {
            Ok(ids) => ids,
            Err(e) => {
                error!("Failed to get device list: {}", e);
                return Err(crate::mobile::MobileError::KdeConnectNotAvailable(
                    "KDE Connect daemon is not running or not accessible".to_string()
                ));
            }
        };

        info!("Found {} KDE Connect devices", device_ids.len());

        let mut devices = Vec::new();
        for device_id in device_ids {
            match self.get_device_info(&device_id) {
                Ok(device_info) => devices.push(device_info),
                Err(e) => {
                    warn!("Failed to get info for device {}: {}", device_id, e);
                }
            }
        }

        Ok(devices)
    }

    pub fn connect_device(&mut self, device_id: String) -> crate::mobile::Result<()> {
        info!("Connecting to device: {}", device_id);

        // Verify device exists and is reachable
        let device_info = self.get_device_info(&device_id)
            .context("Device not found or not reachable")?;

        if !device_info.is_reachable {
            return Err(crate::mobile::MobileError::DeviceNotReachable(
                format!("Device {} is not reachable", device_id)
            ));
        }

        // Check if device supports SMS
        if !device_info.has_sms_plugin {
            warn!("Device {} does not support SMS plugin", device_id);
        }

        self.device_id = Some(device_id);
        info!("Successfully connected to device: {}", device_info.name);

        Ok(())
    }

    pub fn request_conversations(&self) -> crate::mobile::Result<()> {
        let device_id = self.device_id
            .as_ref()
            .ok_or_else(|| crate::mobile::MobileError::DeviceNotPaired("No device connected".to_string()))?;

        debug!("Requesting SMS conversations from device: {}", device_id);

        let path = format!("/modules/kdeconnect/devices/{}", device_id);
        let proxy = self.connection.with_proxy(
            KDECONNECT_SERVICE,
            path,
            self.timeout,
        );

        proxy
            .method_call(
                "org.kde.kdeconnect.device.conversations",
                "requestAllConversations",
                (),
            )
            .context("Failed to request conversations")
            .map_err(|e| crate::mobile::MobileError::MessageSendFailed(e.to_string()))?;

        debug!("Conversation request sent successfully");
        Ok(())
    }

    pub fn send_sms(&self, message: &str, addresses: &[String]) -> crate::mobile::Result<()> {
        let device_id = self.device_id
            .as_ref()
            .ok_or_else(|| crate::mobile::MobileError::DeviceNotPaired("No device connected".to_string()))?;

        info!("Sending SMS to {:?}: {}", addresses, message);

        let path = format!("/modules/kdeconnect/devices/{}", device_id);
        let proxy = self.connection.with_proxy(
            KDECONNECT_SERVICE,
            path,
            self.timeout,
        );

        let addresses_list: Vec<&str> = addresses.iter().map(|s| s.as_str()).collect();

        proxy
            .method_call(
                "org.kde.kdeconnect.device.conversations",
                "sendMessage",
                (message, addresses_list),
            )
            .context("Failed to send SMS message")
            .map_err(|e| crate::mobile::MobileError::MessageSendFailed(e.to_string()))?;

        info!("SMS sent successfully");
        Ok(())
    }

    pub async fn listen_for_messages(&self) -> crate::mobile::Result<mpsc::Receiver<SmsMessage>> {
        let device_id = self.device_id
            .as_ref()
            .ok_or_else(|| crate::mobile::MobileError::DeviceNotPaired("No device connected".to_string()))?
            .clone();

        let (tx, rx) = mpsc::channel(100);
        
        // Create a new connection for the listener to avoid conflicts
        let connection = Connection::new_session()
            .context("Failed to create D-Bus connection for message listening")
            .map_err(|e| crate::mobile::MobileError::DbusConnectionFailed(
                dbus::Error::new_failed(&e.to_string())
            ))?;

        let device_id_clone = device_id.clone();
        let tx_clone = tx.clone();

        tokio::spawn(async move {
            if let Err(e) = Self::message_listener_loop(connection, device_id_clone, tx_clone).await {
                error!("Message listener failed: {}", e);
            }
        });

        Ok(rx)
    }

    pub async fn listen_for_notifications(&self) -> crate::mobile::Result<mpsc::Receiver<MobileNotification>> {
        let device_id = self.device_id
            .as_ref()
            .ok_or_else(|| crate::mobile::MobileError::DeviceNotPaired("No device connected".to_string()))?
            .clone();

        let (tx, rx) = mpsc::channel(100);
        
        // Create a new connection for the listener
        let connection = Connection::new_session()
            .context("Failed to create D-Bus connection for notification listening")
            .map_err(|e| crate::mobile::MobileError::DbusConnectionFailed(
                dbus::Error::new_failed(&e.to_string())
            ))?;

        let device_id_clone = device_id.clone();
        let tx_clone = tx.clone();

        tokio::spawn(async move {
            if let Err(e) = Self::notification_listener_loop(connection, device_id_clone, tx_clone).await {
                error!("Notification listener failed: {}", e);
            }
        });

        Ok(rx)
    }

    async fn message_listener_loop(
        connection: Connection,
        device_id: String,
        tx: mpsc::Sender<SmsMessage>,
    ) -> Result<()> {
        debug!("Starting message listener for device: {}", device_id);

        // Set up D-Bus signal matching for SMS messages
        let match_rule = format!(
            "type='signal',interface='org.kde.kdeconnect.device.conversations',path='/modules/kdeconnect/devices/{}'",
            device_id
        );

        connection
            .add_match_no_cb(&match_rule)
            .context("Failed to add D-Bus match rule for messages")?;

        loop {
            // Poll for D-Bus messages with timeout
            for message in connection.iter(1000) {
                if let Ok(sms_message) = Self::parse_sms_message(&message) {
                    debug!("Received SMS message: {:?}", sms_message);
                    
                    if let Err(e) = tx.send(sms_message).await {
                        error!("Failed to send message to channel: {}", e);
                        break;
                    }
                }
            }

            // Add small delay to prevent busy waiting
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    async fn notification_listener_loop(
        connection: Connection,
        device_id: String,
        tx: mpsc::Sender<MobileNotification>,
    ) -> Result<()> {
        debug!("Starting notification listener for device: {}", device_id);

        // Set up D-Bus signal matching for notifications
        let match_rule = format!(
            "type='signal',interface='org.kde.kdeconnect.device.notifications',path='/modules/kdeconnect/devices/{}'",
            device_id
        );

        connection
            .add_match_no_cb(&match_rule)
            .context("Failed to add D-Bus match rule for notifications")?;

        loop {
            // Poll for D-Bus messages with timeout
            for message in connection.iter(1000) {
                if let Ok(notification) = Self::parse_mobile_notification(&message) {
                    debug!("Received mobile notification: {:?}", notification);
                    
                    if let Err(e) = tx.send(notification).await {
                        error!("Failed to send notification to channel: {}", e);
                        break;
                    }
                }
            }

            // Add small delay to prevent busy waiting
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    fn parse_sms_message(message: &Message) -> Result<SmsMessage> {
        let interface = message.interface()
            .context("Message missing interface")?;

        if interface != "org.kde.kdeconnect.device.conversations" {
            return Err(anyhow::anyhow!("Unexpected interface: {}", interface));
        }

        let member = message.member()
            .context("Message missing member")?;

        match member.as_ref() {
            "conversationMessageReceived" => {
                // Parse message arguments
                let mut args = message.iter_init();
                
                // Extract message data (this is a simplified implementation)
                // Real KDE Connect message parsing would be more complex
                let message_data: HashMap<String, dbus::arg::Variant<Box<dyn dbus::arg::RefArg>>> = 
                    args.read().context("Failed to read message data")?;

                // Extract fields from the message data
                let id = message_data.get("uID")
                    .and_then(dbus_variant_to_i64)
                    .unwrap_or(0) as i32;

                let body = message_data.get("body")
                    .and_then(dbus_variant_to_string)
                    .unwrap_or_default();

                let thread_id = message_data.get("threadID")
                    .and_then(dbus_variant_to_i64)
                    .unwrap_or(0);

                let date = message_data.get("date")
                    .and_then(dbus_variant_to_i64)
                    .unwrap_or_else(|| chrono::Utc::now().timestamp() * 1000);

                let message_type_int = message_data.get("type")
                    .and_then(dbus_variant_to_i64)
                    .unwrap_or(1);

                let message_type = match message_type_int {
                    1 => SmsMessageType::Sms,
                    2 => SmsMessageType::Mms,
                    _ => SmsMessageType::Sms,
                };

                let read = message_data.get("read")
                    .and_then(dbus_variant_to_i64)
                    .map(|r| r != 0)
                    .unwrap_or(false);

                let sub_id = message_data.get("subID")
                    .and_then(dbus_variant_to_i64)
                    .unwrap_or(0);

                // Parse addresses (simplified)
                let addresses = vec!["Unknown".to_string()]; // Would extract from actual message data

                let sms_message = SmsMessage {
                    id,
                    body,
                    addresses,
                    date,
                    message_type,
                    read,
                    thread_id,
                    sub_id,
                    attachments: vec![], // MMS attachments would be parsed here
                };

                Ok(sms_message)
            }
            _ => Err(anyhow::anyhow!("Unhandled message member: {}", member)),
        }
    }

    fn parse_mobile_notification(message: &Message) -> Result<MobileNotification> {
        let interface = message.interface()
            .context("Message missing interface")?;

        if interface != "org.kde.kdeconnect.device.notifications" {
            return Err(anyhow::anyhow!("Unexpected interface: {}", interface));
        }

        let member = message.member()
            .context("Message missing member")?;

        match member.as_ref() {
            "notificationPosted" => {
                let mut args = message.iter_init();
                
                let notification_id: String = args.read()
                    .context("Failed to read notification ID")?;

                let notification_data: HashMap<String, dbus::arg::Variant<Box<dyn dbus::arg::RefArg>>> = 
                    args.read().context("Failed to read notification data")?;

                let app_name = notification_data.get("appName")
                    .and_then(dbus_variant_to_string)
                    .unwrap_or_default();

                let title = notification_data.get("title")
                    .and_then(dbus_variant_to_string)
                    .unwrap_or_default();

                let text = notification_data.get("text")
                    .and_then(dbus_variant_to_string)
                    .unwrap_or_default();

                let time = notification_data.get("time")
                    .and_then(dbus_variant_to_i64)
                    .unwrap_or_else(|| chrono::Utc::now().timestamp() * 1000);

                let dismissable = notification_data.get("dismissable")
                    .and_then(dbus_variant_to_bool)
                    .unwrap_or(true);

                let has_reply_action = notification_data.get("hasReplyAction")
                    .and_then(dbus_variant_to_bool)
                    .unwrap_or(false);

                let reply_id = notification_data.get("replyId")
                    .and_then(dbus_variant_to_string);

                let notification = MobileNotification {
                    id: notification_id,
                    app_name,
                    title,
                    text,
                    icon: None, // Would extract icon data if available
                    time,
                    dismissable,
                    has_reply_action,
                    reply_id,
                    actions: vec![], // Would parse actions if available
                };

                Ok(notification)
            }
            _ => Err(anyhow::anyhow!("Unhandled notification member: {}", member)),
        }
    }

    fn get_device_info(&self, device_id: &str) -> crate::mobile::Result<DeviceInfo> {
        let path = format!("/modules/kdeconnect/devices/{}", device_id);
        let proxy = self.connection.with_proxy(
            KDECONNECT_SERVICE,
            path,
            self.timeout,
        );

        // Get device name
        let name: String = proxy
            .get("org.kde.kdeconnect.device", "name")
            .context("Failed to get device name")
            .map_err(|e| crate::mobile::MobileError::DbusConnectionFailed(
                dbus::Error::new_failed(&e.to_string())
            ))?;

        // Check if device is reachable
        let is_reachable: bool = proxy
            .get("org.kde.kdeconnect.device", "isReachable")
            .context("Failed to get device reachability")
            .map_err(|e| crate::mobile::MobileError::DbusConnectionFailed(
                dbus::Error::new_failed(&e.to_string())
            ))?;

        // Check for plugins
        let plugins: Vec<String> = proxy
            .get("org.kde.kdeconnect.device", "supportedPlugins")
            .unwrap_or_default();

        let has_sms_plugin = plugins.iter().any(|p| p.contains("sms") || p.contains("telephony"));
        let has_notification_plugin = plugins.iter().any(|p| p.contains("notification"));

        // Determine device type from name (heuristic)
        let device_type = if name.to_lowercase().contains("phone") {
            DeviceType::Phone
        } else if name.to_lowercase().contains("tablet") {
            DeviceType::Tablet
        } else if name.to_lowercase().contains("desktop") || name.to_lowercase().contains("laptop") {
            DeviceType::Desktop
        } else {
            DeviceType::Unknown
        };

        Ok(DeviceInfo {
            id: device_id.to_string(),
            name,
            is_reachable,
            has_sms_plugin,
            has_notification_plugin,
            device_type,
        })
    }

    pub fn get_connected_device(&self) -> Option<&String> {
        self.device_id.as_ref()
    }

    pub fn is_connected(&self) -> bool {
        self.device_id.is_some()
    }

    pub fn disconnect(&mut self) {
        if let Some(device_id) = &self.device_id {
            info!("Disconnecting from device: {}", device_id);
            self.device_id = None;
        }
    }

    pub fn send_notification_reply(&self, reply_id: &str, message: &str) -> crate::mobile::Result<()> {
        let device_id = self.device_id
            .as_ref()
            .ok_or_else(|| crate::mobile::MobileError::DeviceNotPaired("No device connected".to_string()))?;

        debug!("Sending notification reply: {} -> {}", reply_id, message);

        let path = format!("/modules/kdeconnect/devices/{}", device_id);
        let proxy = self.connection.with_proxy(
            KDECONNECT_SERVICE,
            path,
            self.timeout,
        );

        proxy
            .method_call(
                "org.kde.kdeconnect.device.notifications",
                "sendReply",
                (reply_id, message),
            )
            .context("Failed to send notification reply")
            .map_err(|e| crate::mobile::MobileError::MessageSendFailed(e.to_string()))?;

        debug!("Notification reply sent successfully");
        Ok(())
    }

    pub fn check_kde_connect_status(&self) -> crate::mobile::Result<bool> {
        debug!("Checking KDE Connect daemon status");

        let proxy = self.connection.with_proxy(
            KDECONNECT_SERVICE,
            KDECONNECT_DAEMON_PATH,
            Duration::from_millis(1000), // Short timeout for status check
        );

        // Try to call a simple method to verify daemon is running
        match proxy.method_call::<(), Vec<String>, _>(KDECONNECT_DAEMON_INTERFACE, "deviceNames", ()) {
            Ok(_) => {
                debug!("KDE Connect daemon is running");
                Ok(true)
            }
            Err(e) => {
                debug!("KDE Connect daemon is not accessible: {}", e);
                Ok(false)
            }
        }
    }
}

impl Drop for KdeConnectClient {
    fn drop(&mut self) {
        if self.device_id.is_some() {
            debug!("KdeConnectClient dropped - cleaning up connection");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kde_connect_client_creation() {
        // Note: This test requires D-Bus session bus to be available
        if std::env::var("SKIP_DBUS_TESTS").is_ok() {
            return;
        }

        let client = KdeConnectClient::new();
        
        // Should succeed or fail gracefully
        match client {
            Ok(client) => {
                assert!(!client.is_connected());
                assert!(client.get_connected_device().is_none());
            }
            Err(e) => {
                // D-Bus might not be available in test environment
                println!("D-Bus not available in test: {}", e);
            }
        }
    }

    #[test]
    fn test_device_connection_without_dbus() {
        // Test the logic without actual D-Bus calls
        if std::env::var("SKIP_DBUS_TESTS").is_ok() {
            return;
        }

        let mut client = KdeConnectClient::new().unwrap();
        
        // Should fail when no device is connected
        let result = client.request_conversations();
        assert!(result.is_err());
        
        let result = client.send_sms("test", &["123".to_string()]);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_message_listening() {
        if std::env::var("SKIP_DBUS_TESTS").is_ok() {
            return;
        }

        let client = KdeConnectClient::new().unwrap();
        
        // Should fail when no device is connected
        let result = client.listen_for_messages().await;
        assert!(result.is_err());
        
        let result = client.listen_for_notifications().await;
        assert!(result.is_err());
    }

    #[test]
    fn test_kde_connect_status_check() {
        if std::env::var("SKIP_DBUS_TESTS").is_ok() {
            return;
        }

        let client = KdeConnectClient::new().unwrap();
        
        // Should return a result (true or false) without crashing
        let status = client.check_kde_connect_status();
        assert!(status.is_ok());
        
        // Log the actual status for debugging
        println!("KDE Connect status: {:?}", status);
    }
}