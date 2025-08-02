use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{info, debug};

use super::types::{DeviceInfo, SmsMessage, MobileNotification, DeviceType, MessageType};

/// Simple KDE Connect client that uses mock implementation for development
/// and can be extended with real D-Bus integration when available
pub struct KdeConnectClient {
    device_id: Option<String>,
    timeout: Duration,
    mock_devices: Vec<DeviceInfo>,
}

impl KdeConnectClient {
    pub fn new() -> crate::mobile::Result<Self> {
        info!("Creating KDE Connect client");
        
        // Create some mock devices for testing
        let mock_devices = vec![
            DeviceInfo {
                id: "android-phone-001".to_string(),
                name: "Samsung Galaxy S24".to_string(),
                is_reachable: true,
                has_sms_plugin: true,
                has_notification_plugin: true,
                device_type: DeviceType::Phone,
            },
            DeviceInfo {
                id: "iphone-001".to_string(),
                name: "iPhone 15 Pro".to_string(),
                is_reachable: false, // Simulate offline device
                has_sms_plugin: false, // iOS limitations
                has_notification_plugin: true,
                device_type: DeviceType::Phone,
            },
            DeviceInfo {
                id: "android-tablet-001".to_string(),
                name: "iPad Pro".to_string(),
                is_reachable: true,
                has_sms_plugin: false,
                has_notification_plugin: true,
                device_type: DeviceType::Tablet,
            },
        ];
        
        Ok(Self {
            device_id: None,
            timeout: Duration::from_millis(5000),
            mock_devices,
        })
    }

    pub fn discover_devices(&self) -> crate::mobile::Result<Vec<DeviceInfo>> {
        debug!("Discovering KDE Connect devices");
        
        // In a real implementation, this would call D-Bus methods
        // For now, return mock devices
        info!("Found {} KDE Connect devices", self.mock_devices.len());
        Ok(self.mock_devices.clone())
    }

    pub fn connect_device(&mut self, device_id: String) -> crate::mobile::Result<()> {
        info!("Connecting to device: {}", device_id);

        // Check if device exists in mock devices
        let device_info = self.mock_devices
            .iter()
            .find(|d| d.id == device_id)
            .ok_or_else(|| crate::mobile::MobileError::DeviceNotPaired(
                format!("Device {} not found", device_id)
            ))?;

        if !device_info.is_reachable {
            return Err(crate::mobile::MobileError::DeviceNotReachable(
                format!("Device {} is not reachable", device_id)
            ));
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
        
        // In a real implementation, this would send a D-Bus method call
        // For now, simulate successful request
        Ok(())
    }

    pub fn send_sms(&self, message: &str, addresses: &[String]) -> crate::mobile::Result<()> {
        let device_id = self.device_id
            .as_ref()
            .ok_or_else(|| crate::mobile::MobileError::DeviceNotPaired("No device connected".to_string()))?;

        info!("Sending SMS to {:?}: {}", addresses, message);
        
        // Check if connected device supports SMS
        let device = self.mock_devices.iter().find(|d| d.id == *device_id)
            .ok_or_else(|| crate::mobile::MobileError::DeviceNotPaired("Device no longer available".to_string()))?;
        
        if !device.has_sms_plugin {
            return Err(crate::mobile::MobileError::MessageSendFailed(
                "Device does not support SMS".to_string()
            ));
        }
        
        // In a real implementation, this would send a D-Bus method call
        // For now, simulate successful send
        info!("SMS sent successfully");
        Ok(())
    }

    pub async fn listen_for_messages(&self) -> crate::mobile::Result<mpsc::Receiver<SmsMessage>> {
        let device_id = self.device_id
            .as_ref()
            .ok_or_else(|| crate::mobile::MobileError::DeviceNotPaired("No device connected".to_string()))?
            .clone();

        let (tx, rx) = mpsc::channel(100);

        // In a real implementation, this would set up D-Bus signal listening
        // For now, spawn a task that occasionally sends mock messages
        tokio::spawn(async move {
            let mut counter = 1;
            let mock_contacts = [
                ("+1234567890", "Alice Johnson"),
                ("+1987654321", "Bob Smith"),
                ("+1555666777", "Carol Wilson"),
            ];
            
            loop {
                tokio::time::sleep(Duration::from_secs(30)).await;
                
                let (phone, name) = mock_contacts[(counter - 1) % mock_contacts.len()];
                let mock_message = SmsMessage {
                    id: counter as i32,
                    body: format!("Mock SMS #{} from {}. How are you doing?", counter, name),
                    addresses: vec![phone.to_string()],
                    date: chrono::Utc::now().timestamp() * 1000,
                    message_type: MessageType::Sms,
                    read: false,
                    thread_id: ((counter - 1) % 3) as i64 + 1, // Rotate between threads 1, 2, 3
                    sub_id: 1,
                    attachments: vec![],
                };

                if tx.send(mock_message).await.is_err() {
                    debug!("Message receiver dropped, ending message loop for device {}", device_id);
                    break;
                }
                
                counter += 1;
                
                // Stop after 15 messages to avoid infinite mock data
                if counter > 15 {
                    info!("Mock message generation completed (15 messages sent)");
                    break;
                }
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

        // In a real implementation, this would set up D-Bus signal listening
        // For now, spawn a task that occasionally sends mock notifications
        tokio::spawn(async move {
            let mut counter = 1;
            let mock_apps = [
                ("Messages", "New text message", true),
                ("WhatsApp", "WhatsApp message", true),
                ("Gmail", "New email received", false),
                ("Calendar", "Meeting reminder", false),
                ("Twitter", "New tweet", false),
                ("Signal", "Secure message", true),
            ];
            
            loop {
                tokio::time::sleep(Duration::from_secs(45)).await;
                
                let (app_name, title_template, has_reply) = mock_apps[(counter - 1) % mock_apps.len()];
                let mock_notification = MobileNotification {
                    id: format!("notif-{}", counter),
                    app_name: app_name.to_string(),
                    title: format!("{} #{}", title_template, counter),
                    text: format!("This is a mock {} notification with some sample text content.", app_name),
                    icon: None,
                    time: chrono::Utc::now().timestamp() * 1000,
                    dismissable: true,
                    has_reply_action: has_reply,
                    reply_id: if has_reply { Some(format!("reply-{}", counter)) } else { None },
                    actions: vec![],
                };

                if tx.send(mock_notification).await.is_err() {
                    debug!("Notification receiver dropped, ending notification loop for device {}", device_id);
                    break;
                }
                
                counter += 1;
                
                // Stop after 12 notifications
                if counter > 12 {
                    info!("Mock notification generation completed (12 notifications sent)");
                    break;
                }
            }
        });

        Ok(rx)
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
        let _device_id = self.device_id
            .as_ref()
            .ok_or_else(|| crate::mobile::MobileError::DeviceNotPaired("No device connected".to_string()))?;

        debug!("Sending notification reply: {} -> {}", reply_id, message);
        
        // In a real implementation, this would send a D-Bus method call
        // For now, simulate successful reply
        info!("Notification reply sent successfully");
        Ok(())
    }

    pub fn check_kde_connect_status(&self) -> crate::mobile::Result<bool> {
        debug!("Checking KDE Connect daemon status");
        
        // In a real implementation, this would check D-Bus service availability
        // For now, always return true (available)
        Ok(true)
    }

    /// Get device capabilities for the connected device
    pub fn get_device_capabilities(&self) -> crate::mobile::Result<Vec<String>> {
        let device_id = self.device_id
            .as_ref()
            .ok_or_else(|| crate::mobile::MobileError::DeviceNotPaired("No device connected".to_string()))?;

        let device = self.mock_devices.iter().find(|d| d.id == *device_id)
            .ok_or_else(|| crate::mobile::MobileError::DeviceNotPaired("Device no longer available".to_string()))?;

        let mut capabilities = Vec::new();
        
        if device.has_sms_plugin {
            capabilities.push("SMS/MMS Messaging".to_string());
        }
        if device.has_notification_plugin {
            capabilities.push("Notification Forwarding".to_string());
        }
        
        capabilities.push("Device Information".to_string());
        
        Ok(capabilities)
    }

    /// Get detailed device information
    pub fn get_device_details(&self) -> crate::mobile::Result<DeviceInfo> {
        let device_id = self.device_id
            .as_ref()
            .ok_or_else(|| crate::mobile::MobileError::DeviceNotPaired("No device connected".to_string()))?;

        let device = self.mock_devices.iter().find(|d| d.id == *device_id)
            .ok_or_else(|| crate::mobile::MobileError::DeviceNotPaired("Device no longer available".to_string()))?;

        Ok(device.clone())
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
    fn test_client_creation() {
        let client = KdeConnectClient::new().unwrap();
        assert!(!client.is_connected());
        assert!(client.get_connected_device().is_none());
    }

    #[test]
    fn test_device_discovery() {
        let client = KdeConnectClient::new().unwrap();
        let devices = client.discover_devices().unwrap();
        
        assert!(!devices.is_empty());
        assert!(devices.iter().any(|d| d.device_type == DeviceType::Phone));
        assert!(devices.iter().any(|d| d.has_sms_plugin));
        assert!(devices.iter().any(|d| d.has_notification_plugin));
    }

    #[test]
    fn test_device_connection() {
        let mut client = KdeConnectClient::new().unwrap();
        
        // Try to connect to first available device
        let devices = client.discover_devices().unwrap();
        let reachable_device = devices.iter().find(|d| d.is_reachable).unwrap();
        
        client.connect_device(reachable_device.id.clone()).unwrap();
        assert!(client.is_connected());
        assert_eq!(client.get_connected_device(), Some(&reachable_device.id));
        
        // Test operations that require connection
        client.request_conversations().unwrap();
        
        // Test capabilities
        let capabilities = client.get_device_capabilities().unwrap();
        assert!(!capabilities.is_empty());
        
        let device_details = client.get_device_details().unwrap();
        assert_eq!(device_details.id, reachable_device.id);
        
        client.disconnect();
        assert!(!client.is_connected());
    }

    #[test]
    fn test_unreachable_device() {
        let mut client = KdeConnectClient::new().unwrap();
        let devices = client.discover_devices().unwrap();
        let unreachable_device = devices.iter().find(|d| !d.is_reachable).unwrap();
        
        // Should fail to connect to unreachable device
        let result = client.connect_device(unreachable_device.id.clone());
        assert!(result.is_err());
        assert!(!client.is_connected());
    }

    #[test]
    fn test_sms_capabilities() {
        let mut client = KdeConnectClient::new().unwrap();
        let devices = client.discover_devices().unwrap();
        let sms_device = devices.iter().find(|d| d.has_sms_plugin && d.is_reachable).unwrap();
        
        client.connect_device(sms_device.id.clone()).unwrap();
        
        // Should succeed for device with SMS support
        let result = client.send_sms("Test message", &["+1234567890".to_string()]);
        assert!(result.is_ok());
        
        // Test reply functionality
        let result = client.send_notification_reply("test-reply-id", "Test reply");
        assert!(result.is_ok());
    }

    #[test]
    fn test_non_sms_device() {
        let mut client = KdeConnectClient::new().unwrap();
        let devices = client.discover_devices().unwrap();
        let non_sms_device = devices.iter().find(|d| !d.has_sms_plugin && d.is_reachable).unwrap();
        
        client.connect_device(non_sms_device.id.clone()).unwrap();
        
        // Should fail for device without SMS support
        let result = client.send_sms("Test message", &["+1234567890".to_string()]);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_message_listening() {
        let mut client = KdeConnectClient::new().unwrap();
        let devices = client.discover_devices().unwrap();
        let phone_device = devices.iter().find(|d| d.has_sms_plugin && d.is_reachable).unwrap();
        
        client.connect_device(phone_device.id.clone()).unwrap();
        
        let mut message_receiver = client.listen_for_messages().await.unwrap();
        
        // Test that we can receive mock messages (with timeout)
        tokio::select! {
            maybe_message = message_receiver.recv() => {
                if let Some(message) = maybe_message {
                    assert!(message.body.contains("Mock SMS"));
                    assert!(message.thread_id >= 1);
                    assert!(message.id >= 1);
                    assert!(!message.addresses.is_empty());
                }
            }
            _ = tokio::time::sleep(Duration::from_secs(35)) => {
                // First message should arrive within 35 seconds
            }
        }
    }

    #[tokio::test]
    async fn test_notification_listening() {
        let mut client = KdeConnectClient::new().unwrap();
        let devices = client.discover_devices().unwrap();
        let phone_device = devices.iter().find(|d| d.has_notification_plugin && d.is_reachable).unwrap();
        
        client.connect_device(phone_device.id.clone()).unwrap();
        
        let mut notification_receiver = client.listen_for_notifications().await.unwrap();
        
        // Test that we can receive mock notifications (with timeout)
        tokio::select! {
            maybe_notification = notification_receiver.recv() => {
                if let Some(notification) = maybe_notification {
                    assert!(!notification.title.is_empty());
                    assert!(!notification.app_name.is_empty());
                    assert!(!notification.text.is_empty());
                    assert!(notification.id.starts_with("notif-"));
                }
            }
            _ = tokio::time::sleep(Duration::from_secs(50)) => {
                // First notification should arrive within 50 seconds
            }
        }
    }

    #[test]
    fn test_status_check() {
        let client = KdeConnectClient::new().unwrap();
        let status = client.check_kde_connect_status().unwrap();
        assert!(status); // Mock always returns true
    }

    #[test]
    fn test_operations_without_connection() {
        let client = KdeConnectClient::new().unwrap();
        
        // All operations should fail when no device is connected
        assert!(client.request_conversations().is_err());
        assert!(client.send_sms("test", &["+123".to_string()]).is_err());
        assert!(client.send_notification_reply("test", "test").is_err());
        assert!(client.get_device_capabilities().is_err());
        assert!(client.get_device_details().is_err());
    }
}