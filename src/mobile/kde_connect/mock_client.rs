use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{info, warn, debug};

use super::types::{DeviceInfo, SmsMessage, MobileNotification, DeviceType, MessageType, ContactInfo};

/// Mock implementation of KDE Connect client for testing and development
/// when D-Bus is not available
pub struct MockKdeConnectClient {
    device_id: Option<String>,
    timeout: Duration,
    mock_devices: Vec<DeviceInfo>,
}

impl MockKdeConnectClient {
    pub fn new() -> crate::mobile::Result<Self> {
        info!("Creating mock KDE Connect client (D-Bus not available)");
        
        // Create some mock devices for testing
        let mock_devices = vec![
            DeviceInfo {
                id: "mock-phone-001".to_string(),
                name: "Mock Android Phone".to_string(),
                is_reachable: true,
                has_sms_plugin: true,
                has_notification_plugin: true,
                device_type: DeviceType::Phone,
            },
            DeviceInfo {
                id: "mock-tablet-001".to_string(),
                name: "Mock Tablet".to_string(),
                is_reachable: false,
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
        debug!("Mock: Discovering KDE Connect devices");
        info!("Mock: Found {} devices", self.mock_devices.len());
        Ok(self.mock_devices.clone())
    }

    pub fn connect_device(&mut self, device_id: String) -> crate::mobile::Result<()> {
        info!("Mock: Connecting to device: {}", device_id);

        // Check if device exists in mock devices
        let device_info = self.mock_devices
            .iter()
            .find(|d| d.id == device_id)
            .ok_or_else(|| crate::mobile::MobileError::DeviceNotPaired(
                format!("Mock device {} not found", device_id)
            ))?;

        if !device_info.is_reachable {
            return Err(crate::mobile::MobileError::DeviceNotReachable(
                format!("Mock device {} is not reachable", device_id)
            ));
        }

        self.device_id = Some(device_id);
        info!("Mock: Successfully connected to device: {}", device_info.name);

        Ok(())
    }

    pub fn request_conversations(&self) -> crate::mobile::Result<()> {
        let device_id = self.device_id
            .as_ref()
            .ok_or_else(|| crate::mobile::MobileError::DeviceNotPaired("No device connected".to_string()))?;

        debug!("Mock: Requesting SMS conversations from device: {}", device_id);
        
        // Simulate successful request
        Ok(())
    }

    pub fn send_sms(&self, message: &str, addresses: &[String]) -> crate::mobile::Result<()> {
        let device_id = self.device_id
            .as_ref()
            .ok_or_else(|| crate::mobile::MobileError::DeviceNotPaired("No device connected".to_string()))?;

        info!("Mock: Sending SMS to {:?}: {}", addresses, message);
        
        // Simulate successful send
        info!("Mock: SMS sent successfully");
        Ok(())
    }

    pub async fn listen_for_messages(&self) -> crate::mobile::Result<mpsc::Receiver<SmsMessage>> {
        let device_id = self.device_id
            .as_ref()
            .ok_or_else(|| crate::mobile::MobileError::DeviceNotPaired("No device connected".to_string()))?
            .clone();

        let (tx, rx) = mpsc::channel(100);

        // Spawn a task that occasionally sends mock messages
        tokio::spawn(async move {
            let mut counter = 1;
            loop {
                tokio::time::sleep(Duration::from_secs(30)).await;
                
                let mock_message = SmsMessage {
                    id: counter,
                    body: format!("Mock SMS message #{}", counter),
                    addresses: vec!["+1234567890".to_string()],
                    date: chrono::Utc::now().timestamp() * 1000,
                    message_type: MessageType::Sms,
                    read: false,
                    thread_id: 1,
                    sub_id: 1,
                    attachments: vec![],
                };

                if tx.send(mock_message).await.is_err() {
                    debug!("Mock: Message receiver dropped, ending mock message loop");
                    break;
                }
                
                counter += 1;
                
                // Stop after 10 messages to avoid infinite mock data
                if counter > 10 {
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

        // Spawn a task that occasionally sends mock notifications
        tokio::spawn(async move {
            let mut counter = 1;
            let mock_apps = ["Messages", "WhatsApp", "Gmail", "Calendar"];
            
            loop {
                tokio::time::sleep(Duration::from_secs(45)).await;
                
                let app_name = mock_apps[(counter - 1) % mock_apps.len()];
                let mock_notification = MobileNotification {
                    id: format!("mock-notif-{}", counter),
                    app_name: app_name.to_string(),
                    title: format!("Mock {} Notification", app_name),
                    text: format!("This is mock notification #{} from {}", counter, app_name),
                    icon: None,
                    time: chrono::Utc::now().timestamp() * 1000,
                    dismissable: true,
                    has_reply_action: app_name == "Messages" || app_name == "WhatsApp",
                    reply_id: if app_name == "Messages" { Some(format!("reply-{}", counter)) } else { None },
                    actions: vec![],
                };

                if tx.send(mock_notification).await.is_err() {
                    debug!("Mock: Notification receiver dropped, ending mock notification loop");
                    break;
                }
                
                counter += 1;
                
                // Stop after 8 notifications
                if counter > 8 {
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
            info!("Mock: Disconnecting from device: {}", device_id);
            self.device_id = None;
        }
    }

    pub fn send_notification_reply(&self, reply_id: &str, message: &str) -> crate::mobile::Result<()> {
        let _device_id = self.device_id
            .as_ref()
            .ok_or_else(|| crate::mobile::MobileError::DeviceNotPaired("No device connected".to_string()))?;

        debug!("Mock: Sending notification reply: {} -> {}", reply_id, message);
        info!("Mock: Notification reply sent successfully");
        Ok(())
    }

    pub fn check_kde_connect_status(&self) -> crate::mobile::Result<bool> {
        debug!("Mock: Checking KDE Connect daemon status");
        // Mock always returns true (daemon available)
        Ok(true)
    }
}

impl Drop for MockKdeConnectClient {
    fn drop(&mut self) {
        if self.device_id.is_some() {
            debug!("Mock: KdeConnectClient dropped - cleaning up connection");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_client_creation() {
        let client = MockKdeConnectClient::new().unwrap();
        assert!(!client.is_connected());
        assert!(client.get_connected_device().is_none());
    }

    #[test]
    fn test_mock_device_discovery() {
        let client = MockKdeConnectClient::new().unwrap();
        let devices = client.discover_devices().unwrap();
        
        assert!(!devices.is_empty());
        assert!(devices.iter().any(|d| d.device_type == DeviceType::Phone));
        assert!(devices.iter().any(|d| d.has_sms_plugin));
    }

    #[test]
    fn test_mock_device_connection() {
        let mut client = MockKdeConnectClient::new().unwrap();
        
        // Try to connect to first available device
        let devices = client.discover_devices().unwrap();
        let phone_device = devices.iter().find(|d| d.device_type == DeviceType::Phone).unwrap();
        
        client.connect_device(phone_device.id.clone()).unwrap();
        assert!(client.is_connected());
        assert_eq!(client.get_connected_device(), Some(&phone_device.id));
        
        // Test operations that require connection
        client.request_conversations().unwrap();
        client.send_sms("Test message", &["+1234567890".to_string()]).unwrap();
        
        client.disconnect();
        assert!(!client.is_connected());
    }

    #[test]
    fn test_mock_unreachable_device() {
        let mut client = MockKdeConnectClient::new().unwrap();
        let devices = client.discover_devices().unwrap();
        let unreachable_device = devices.iter().find(|d| !d.is_reachable).unwrap();
        
        // Should fail to connect to unreachable device
        let result = client.connect_device(unreachable_device.id.clone());
        assert!(result.is_err());
        assert!(!client.is_connected());
    }

    #[tokio::test]
    async fn test_mock_message_listening() {
        let mut client = MockKdeConnectClient::new().unwrap();
        let devices = client.discover_devices().unwrap();
        let phone_device = devices.iter().find(|d| d.has_sms_plugin).unwrap();
        
        client.connect_device(phone_device.id.clone()).unwrap();
        
        let mut message_receiver = client.listen_for_messages().await.unwrap();
        let mut notification_receiver = client.listen_for_notifications().await.unwrap();
        
        // Test that we can receive mock messages (with timeout)
        tokio::select! {
            maybe_message = message_receiver.recv() => {
                if let Some(message) = maybe_message {
                    assert!(message.body.contains("Mock SMS"));
                    assert_eq!(message.thread_id, 1);
                }
            }
            _ = tokio::time::sleep(Duration::from_secs(35)) => {
                // First message should arrive within 35 seconds
            }
        }
        
        tokio::select! {
            maybe_notification = notification_receiver.recv() => {
                if let Some(notification) = maybe_notification {
                    assert!(notification.title.contains("Mock"));
                    assert!(!notification.app_name.is_empty());
                }
            }
            _ = tokio::time::sleep(Duration::from_secs(50)) => {
                // First notification should arrive within 50 seconds
            }
        }
    }

    #[test]
    fn test_mock_status_check() {
        let client = MockKdeConnectClient::new().unwrap();
        let status = client.check_kde_connect_status().unwrap();
        assert!(status); // Mock always returns true
    }
}