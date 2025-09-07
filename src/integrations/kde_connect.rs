use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command as AsyncCommand;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KdeConnectConfig {
    pub enabled: bool,
    pub device_id: Option<String>,
    pub device_name: Option<String>,
    pub notification_types: Vec<String>,
    pub auto_pair: bool,
    pub sound_enabled: bool,
}

impl Default for KdeConnectConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            device_id: None,
            device_name: None,
            notification_types: vec![
                "new_email".to_string(),
                "calendar_reminder".to_string(),
                "sync_complete".to_string(),
            ],
            auto_pair: false,
            sound_enabled: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct KdeConnectDevice {
    pub id: String,
    pub name: String,
    pub paired: bool,
    pub reachable: bool,
}

pub struct KdeConnectIntegration {
    config: KdeConnectConfig,
}

impl KdeConnectIntegration {
    pub fn new(config: KdeConnectConfig) -> Self {
        Self { config }
    }

    /// Check if KDE Connect CLI is available on the system
    pub async fn is_available() -> bool {
        AsyncCommand::new("kdeconnect-cli")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .map(|status| status.success())
            .unwrap_or(false)
    }

    /// Initialize KDE Connect integration
    pub async fn initialize(&mut self) -> Result<()> {
        if !self.config.enabled {
            tracing::debug!("KDE Connect integration disabled");
            return Ok(());
        }

        if !Self::is_available().await {
            return Err(anyhow!(
                "KDE Connect CLI not found. Please install kdeconnect."
            ));
        }

        // Refresh devices
        self.refresh_devices().await?;

        if let Some(device_id) = &self.config.device_id {
            if self.is_device_available(device_id).await? {
                tracing::info!(
                    "✅ KDE Connect initialized with device: {} ({})",
                    device_id,
                    self.config.device_name.as_deref().unwrap_or("Unknown")
                );
            } else {
                tracing::warn!(
                    "⚠️ Configured KDE Connect device not available: {}",
                    device_id
                );
            }
        } else {
            tracing::info!("KDE Connect enabled but no device configured");
        }

        Ok(())
    }

    /// Refresh available devices
    async fn refresh_devices(&self) -> Result<()> {
        let output = AsyncCommand::new("kdeconnect-cli")
            .arg("--refresh")
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .status()
            .await?;

        if !output.success() {
            return Err(anyhow!("Failed to refresh KDE Connect devices"));
        }

        // Give a moment for devices to be discovered
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        Ok(())
    }

    /// Check if a specific device is available and reachable
    async fn is_device_available(&self, device_id: &str) -> Result<bool> {
        let output = AsyncCommand::new("kdeconnect-cli")
            .arg("--device-id")
            .arg(device_id)
            .arg("--available")
            .output()
            .await?;

        Ok(output.status.success())
    }

    /// List all available devices
    pub async fn list_devices() -> Result<Vec<KdeConnectDevice>> {
        let mut cmd = AsyncCommand::new("kdeconnect-cli");
        cmd.arg("--list-available");
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let mut child = cmd.spawn()?;
        let stdout = child.stdout.take().unwrap();
        let mut reader = BufReader::new(stdout);
        let mut devices = Vec::new();
        let mut line = String::new();

        while reader.read_line(&mut line).await? > 0 {
            let trimmed = line.trim();
            if !trimmed.is_empty() && trimmed.contains(':') {
                if let Some(device) = Self::parse_device_line(trimmed) {
                    devices.push(device);
                }
            }
            line.clear();
        }

        let _ = child.wait().await;
        Ok(devices)
    }

    /// Parse a device line from kdeconnect-cli output
    fn parse_device_line(line: &str) -> Option<KdeConnectDevice> {
        // Expected format: "- DeviceName: device-id (paired and reachable)"
        // or: "- DeviceName: device-id (paired)"
        // or: "- DeviceName: device-id (reachable)"

        if let Some(colon_pos) = line.find(':') {
            let name_part = line[..colon_pos].trim_start_matches("- ").trim();
            let rest = &line[colon_pos + 1..].trim();

            if let Some(paren_pos) = rest.find('(') {
                let device_id = rest[..paren_pos].trim();
                let status = &rest[paren_pos..];

                let paired = status.contains("paired");
                let reachable = status.contains("reachable");

                return Some(KdeConnectDevice {
                    id: device_id.to_string(),
                    name: name_part.to_string(),
                    paired,
                    reachable,
                });
            }
        }

        None
    }

    /// Send email notification to configured device
    pub async fn send_email_notification(
        &self,
        sender: &str,
        subject: &str,
        preview: &str,
    ) -> Result<()> {
        if !self.config.enabled
            || !self
                .config
                .notification_types
                .contains(&"new_email".to_string())
        {
            return Ok(()); // Silently skip if not enabled or configured
        }

        let device_id = self
            .config
            .device_id
            .as_ref()
            .ok_or_else(|| anyhow!("No KDE Connect device configured"))?;

        // Create notification content
        let title = format!("📧 New Email from {}", sender);
        let body = format!(
            "Subject: {}\n\n{}",
            subject,
            if preview.len() > 100 {
                format!("{}...", &preview[..97])
            } else {
                preview.to_string()
            }
        );

        self.send_notification_internal(device_id, &title, &body)
            .await?;
        tracing::info!(
            "📱 Sent email notification to KDE Connect device: {}",
            sender
        );
        Ok(())
    }

    /// Send custom notification
    pub async fn send_notification(&self, title: &str, message: &str) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let device_id = self
            .config
            .device_id
            .as_ref()
            .ok_or_else(|| anyhow!("No KDE Connect device configured"))?;

        self.send_notification_internal(device_id, title, message)
            .await?;
        tracing::debug!("📱 Sent notification via KDE Connect: {}", title);
        Ok(())
    }

    /// Internal method to send notification
    async fn send_notification_internal(
        &self,
        device_id: &str,
        title: &str,
        message: &str,
    ) -> Result<()> {
        let notification_text = format!("{}\n{}", title, message);

        let output = AsyncCommand::new("kdeconnect-cli")
            .arg("--device-id")
            .arg(device_id)
            .arg("--ping-msg")
            .arg(&notification_text)
            .output()
            .await?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!(
                "Failed to send KDE Connect notification: {}",
                error
            ));
        }

        Ok(())
    }

    /// Find your phone (make it ring)
    pub async fn find_phone(&self) -> Result<()> {
        if !self.config.enabled {
            return Err(anyhow!("KDE Connect not enabled"));
        }

        let device_id = self
            .config
            .device_id
            .as_ref()
            .ok_or_else(|| anyhow!("No KDE Connect device configured"))?;

        let output = AsyncCommand::new("kdeconnect-cli")
            .arg("--device-id")
            .arg(device_id)
            .arg("--ring")
            .output()
            .await?;

        if output.status.success() {
            tracing::info!("📱 Triggered find phone on KDE Connect device");
            Ok(())
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            Err(anyhow!("Failed to trigger find phone: {}", error))
        }
    }

    /// Share file with configured device
    pub async fn share_file(&self, file_path: &str) -> Result<()> {
        if !self.config.enabled {
            return Err(anyhow!("KDE Connect not enabled"));
        }

        let device_id = self
            .config
            .device_id
            .as_ref()
            .ok_or_else(|| anyhow!("No KDE Connect device configured"))?;

        // Check if file exists
        if !std::path::Path::new(file_path).exists() {
            return Err(anyhow!("File does not exist: {}", file_path));
        }

        let output = AsyncCommand::new("kdeconnect-cli")
            .arg("--device-id")
            .arg(device_id)
            .arg("--share")
            .arg(file_path)
            .output()
            .await?;

        if output.status.success() {
            tracing::info!("📱 Shared file via KDE Connect: {}", file_path);
            Ok(())
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            Err(anyhow!("Failed to share file via KDE Connect: {}", error))
        }
    }

    /// Pair with a device
    pub async fn pair_device(device_id: &str) -> Result<()> {
        let output = AsyncCommand::new("kdeconnect-cli")
            .arg("--device-id")
            .arg(device_id)
            .arg("--pair")
            .output()
            .await?;

        if output.status.success() {
            tracing::info!("📱 Initiated pairing with device: {}", device_id);
            Ok(())
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            Err(anyhow!("Failed to pair with device: {}", error))
        }
    }

    /// Unpair from a device
    pub async fn unpair_device(device_id: &str) -> Result<()> {
        let output = AsyncCommand::new("kdeconnect-cli")
            .arg("--device-id")
            .arg(device_id)
            .arg("--unpair")
            .output()
            .await?;

        if output.status.success() {
            tracing::info!("📱 Unpaired from device: {}", device_id);
            Ok(())
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            Err(anyhow!("Failed to unpair from device: {}", error))
        }
    }

    /// Check if integration is enabled and properly configured
    pub fn is_enabled(&self) -> bool {
        self.config.enabled && self.config.device_id.is_some()
    }

    /// Get current configuration
    pub fn config(&self) -> &KdeConnectConfig {
        &self.config
    }

    /// Update configuration
    pub fn update_config(&mut self, config: KdeConnectConfig) {
        self.config = config;
    }

    /// Send calendar reminder
    pub async fn send_calendar_reminder(&self, event_title: &str, start_time: &str) -> Result<()> {
        if !self.config.enabled
            || !self
                .config
                .notification_types
                .contains(&"calendar_reminder".to_string())
        {
            return Ok(());
        }

        let title = "📅 Calendar Reminder";
        let message = format!("Event: {}\nTime: {}", event_title, start_time);

        self.send_notification(title, &message).await
    }

    /// Send sync completion notification
    pub async fn send_sync_complete(&self, account: &str, new_emails: usize) -> Result<()> {
        if !self.config.enabled
            || !self
                .config
                .notification_types
                .contains(&"sync_complete".to_string())
        {
            return Ok(());
        }

        let title = "🔄 Sync Complete";
        let message = if new_emails > 0 {
            format!("Account: {}\n{} new emails received", account, new_emails)
        } else {
            format!("Account: {}\nNo new emails", account)
        };

        self.send_notification(title, &message).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_device_line() {
        let line = "- My Phone: abc123def (paired and reachable)";
        let device = KdeConnectIntegration::parse_device_line(line).unwrap();

        assert_eq!(device.name, "My Phone");
        assert_eq!(device.id, "abc123def");
        assert!(device.paired);
        assert!(device.reachable);
    }

    #[test]
    fn test_parse_device_line_paired_only() {
        let line = "- Tablet: xyz789 (paired)";
        let device = KdeConnectIntegration::parse_device_line(line).unwrap();

        assert_eq!(device.name, "Tablet");
        assert_eq!(device.id, "xyz789");
        assert!(device.paired);
        assert!(!device.reachable);
    }

    #[tokio::test]
    async fn test_is_available() {
        // This test will pass if KDE Connect is installed, fail otherwise
        // In CI/CD, this would be skipped or mocked
        println!(
            "KDE Connect available: {}",
            KdeConnectIntegration::is_available().await
        );
    }
}
