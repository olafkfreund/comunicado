use crate::imap::{ImapConfig, ImapError, ImapResult};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader as AsyncBufReader, BufWriter as AsyncBufWriter};
use tokio::net::TcpStream as AsyncTcpStream;
use tokio::time::timeout;

/// IMAP connection state
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connected,
    Authenticated,
    Selected(String), // Selected folder name
}

/// IMAP connection wrapper
pub struct ImapConnection {
    config: ImapConfig,
    state: ConnectionState,
    stream: Option<AsyncTcpStream>,
    reader: Option<AsyncBufReader<tokio::net::tcp::OwnedReadHalf>>,
    writer: Option<AsyncBufWriter<tokio::net::tcp::OwnedWriteHalf>>,
    tag_counter: u32,
    greeting: Option<String>,
}

impl ImapConnection {
    /// Create a new IMAP connection
    pub fn new(config: ImapConfig) -> Self {
        Self {
            config,
            state: ConnectionState::Disconnected,
            stream: None,
            reader: None,
            writer: None,
            tag_counter: 0,
            greeting: None,
        }
    }
    
    /// Connect to the IMAP server
    pub async fn connect(&mut self) -> ImapResult<()> {
        if self.state != ConnectionState::Disconnected {
            return Err(ImapError::invalid_state("Already connected"));
        }
        
        // Create socket address
        let addr = format!("{}:{}", self.config.hostname, self.config.port);
        let socket_addrs: Vec<_> = addr.to_socket_addrs()
            .map_err(|e| ImapError::connection(format!("Failed to resolve address {}: {}", addr, e)))?
            .collect();
            
        if socket_addrs.is_empty() {
            return Err(ImapError::connection(format!("No addresses found for {}", addr)));
        }
        
        // Connect with timeout
        let timeout_duration = Duration::from_secs(self.config.timeout_seconds);
        let stream = timeout(timeout_duration, AsyncTcpStream::connect(&socket_addrs[0]))
            .await
            .map_err(|_| ImapError::Timeout)?
            .map_err(|e| ImapError::connection(format!("Failed to connect to {}: {}", addr, e)))?;
            
        // TODO: Add TLS support here when needed
        if self.config.use_tls {
            return Err(ImapError::not_supported("TLS support not yet implemented"));
        }
        
        // Split stream for reading and writing
        let (read_half, write_half) = stream.into_split();
        let reader = AsyncBufReader::new(read_half);
        let writer = AsyncBufWriter::new(write_half);
        
        // For now, store a reference to be used later
        // This is a simplified approach - in production we'd handle this differently
        self.reader = Some(reader);
        self.writer = Some(writer);
        
        // Read greeting
        let greeting = self.read_response().await?;
        self.greeting = Some(greeting.clone());
        
        // Check if server sent OK greeting
        if !greeting.starts_with("* OK") && !greeting.starts_with("* PREAUTH") {
            return Err(ImapError::server(format!("Invalid greeting: {}", greeting)));
        }
        
        // Update state
        if greeting.starts_with("* PREAUTH") {
            self.state = ConnectionState::Authenticated;
        } else {
            self.state = ConnectionState::Connected;
        }
        
        Ok(())
    }
    
    /// Disconnect from the IMAP server
    pub async fn disconnect(&mut self) -> ImapResult<()> {
        if self.state == ConnectionState::Disconnected {
            return Ok(());
        }
        
        // Send LOGOUT command if connected
        if let Err(e) = self.send_command("LOGOUT").await {
            tracing::warn!("Failed to send LOGOUT command: {}", e);
        }
        
        // Clean up connection
        self.reader = None;
        self.writer = None;
        self.stream = None;
        self.state = ConnectionState::Disconnected;
        self.tag_counter = 0;
        self.greeting = None;
        
        Ok(())
    }
    
    /// Send a command to the server
    pub async fn send_command(&mut self, command: &str) -> ImapResult<String> {
        if self.state == ConnectionState::Disconnected {
            return Err(ImapError::invalid_state("Not connected"));
        }
        
        let writer = self.writer.as_mut()
            .ok_or_else(|| ImapError::invalid_state("No writer available"))?;
        
        // Generate unique tag
        self.tag_counter += 1;
        let tag = format!("A{:04}", self.tag_counter);
        
        // Send command
        let full_command = format!("{} {}\r\n", tag, command);
        writer.write_all(full_command.as_bytes()).await
            .map_err(|e| ImapError::connection(format!("Failed to send command: {}", e)))?;
        writer.flush().await
            .map_err(|e| ImapError::connection(format!("Failed to flush command: {}", e)))?;
        
        // Read response until we get the tagged response
        let mut responses = Vec::new();
        loop {
            let line = self.read_response().await?;
            responses.push(line.clone());
            
            if line.starts_with(&tag) {
                // This is our tagged response
                if line.starts_with(&format!("{} OK", tag)) {
                    break;
                } else if line.starts_with(&format!("{} NO", tag)) {
                    return Err(ImapError::server(format!("Command failed: {}", line)));
                } else if line.starts_with(&format!("{} BAD", tag)) {
                    return Err(ImapError::protocol(format!("Bad command: {}", line)));
                }
            }
        }
        
        Ok(responses.join("\n"))
    }
    
    /// Read a single response line from the server  
    async fn read_response(&mut self) -> ImapResult<String> {
        let reader = self.reader.as_mut()
            .ok_or_else(|| ImapError::invalid_state("No reader available"))?;
        
        let mut line = String::new();
        let timeout_duration = Duration::from_secs(self.config.timeout_seconds);
        
        timeout(timeout_duration, reader.read_line(&mut line))
            .await
            .map_err(|_| ImapError::Timeout)?
            .map_err(|e| ImapError::connection(format!("Failed to read response: {}", e)))?;
        
        // Remove trailing CRLF
        if line.ends_with("\r\n") {
            line.truncate(line.len() - 2);
        } else if line.ends_with('\n') {
            line.truncate(line.len() - 1);
        }
        
        Ok(line)
    }
    
    /// Get current connection state
    pub fn state(&self) -> &ConnectionState {
        &self.state
    }
    
    /// Get server greeting
    pub fn greeting(&self) -> Option<&String> {
        self.greeting.as_ref()
    }
    
    /// Check if connected
    pub fn is_connected(&self) -> bool {
        self.state != ConnectionState::Disconnected
    }
    
    /// Check if authenticated
    pub fn is_authenticated(&self) -> bool {
        matches!(self.state, ConnectionState::Authenticated | ConnectionState::Selected(_))
    }
    
    /// Get selected folder (if any)
    pub fn selected_folder(&self) -> Option<&String> {
        match &self.state {
            ConnectionState::Selected(folder) => Some(folder),
            _ => None,
        }
    }
    
    /// Update connection state
    pub(crate) fn set_state(&mut self, state: ConnectionState) {
        self.state = state;
    }
    
    /// Get configuration
    pub fn config(&self) -> &ImapConfig {
        &self.config
    }
}

impl Drop for ImapConnection {
    fn drop(&mut self) {
        // Clean up connection in destructor
        // Note: This is synchronous cleanup - in async context we'd need explicit disconnect
        self.reader = None;
        self.writer = None;
        self.stream = None;
        self.state = ConnectionState::Disconnected;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_connection_creation() {
        let config = ImapConfig::new(
            "imap.example.com".to_string(),
            993,
            "user@example.com".to_string(),
            "password".to_string(),
        );
        
        let conn = ImapConnection::new(config);
        assert_eq!(conn.state(), &ConnectionState::Disconnected);
        assert!(!conn.is_connected());
        assert!(!conn.is_authenticated());
        assert!(conn.selected_folder().is_none());
    }
    
    #[test]
    fn test_predefined_configs() {
        let gmail_config = ImapConfig::gmail("user@gmail.com".to_string(), "password".to_string());
        assert_eq!(gmail_config.hostname, "imap.gmail.com");
        assert_eq!(gmail_config.port, 993);
        assert!(gmail_config.use_tls);
        
        let outlook_config = ImapConfig::outlook("user@outlook.com".to_string(), "password".to_string());
        assert_eq!(outlook_config.hostname, "outlook.office365.com");
        assert_eq!(outlook_config.port, 993);
        assert!(outlook_config.use_tls);
    }
}