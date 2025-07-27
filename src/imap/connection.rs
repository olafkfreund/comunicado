use crate::imap::{ImapConfig, ImapError, ImapResult};
use std::net::ToSocketAddrs;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader as AsyncBufReader, BufWriter as AsyncBufWriter};
use tokio::net::TcpStream as AsyncTcpStream;
use tokio::time::timeout;
use tokio_rustls::{TlsConnector, client::TlsStream};
use rustls::{ClientConfig, RootCertStore};
use base64::prelude::*;

/// Connection stream that can be either plain TCP or TLS
enum ConnectionStream {
    Plain(AsyncTcpStream),
    Tls(TlsStream<AsyncTcpStream>),
}

/// Split connection stream for reading/writing
enum SplitStream {
    Plain {
        reader: AsyncBufReader<tokio::net::tcp::OwnedReadHalf>,
        writer: AsyncBufWriter<tokio::net::tcp::OwnedWriteHalf>,
    },
    Tls {
        reader: AsyncBufReader<tokio::io::ReadHalf<TlsStream<AsyncTcpStream>>>,
        writer: AsyncBufWriter<tokio::io::WriteHalf<TlsStream<AsyncTcpStream>>>,
    },
}

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
    stream: Option<SplitStream>,
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
            tag_counter: 0,
            greeting: None,
        }
    }
    
    /// Connect to the IMAP server
    pub async fn connect(&mut self) -> ImapResult<()> {
        println!("DEBUG: ImapConnection::connect() called");
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
        tracing::info!("Attempting TCP connection to {} (timeout: {}s)", addr, self.config.timeout_seconds);
        
        let tcp_stream = timeout(timeout_duration, AsyncTcpStream::connect(&socket_addrs[0]))
            .await
            .map_err(|_| {
                tracing::error!("TCP connection to {} timed out after {}s", addr, self.config.timeout_seconds);
                ImapError::Timeout
            })?
            .map_err(|e| {
                tracing::error!("TCP connection to {} failed: {}", addr, e);
                ImapError::connection(format!("Failed to connect to {}: {}", addr, e))
            })?;
        
        tracing::info!("TCP connection to {} established successfully", addr);
        
        let split_stream = if self.config.use_tls {
            // Set up TLS connection
            println!("DEBUG: Starting TLS handshake with {}", addr);
            tracing::info!("Starting TLS handshake with {}", addr);
            let mut root_store = RootCertStore::empty();
            root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
            
            let config = ClientConfig::builder()
                .with_root_certificates(root_store)
                .with_no_client_auth();
            
            let connector = TlsConnector::from(Arc::new(config));
            let hostname = self.config.hostname.clone(); // Clone to avoid lifetime issues
            let domain = rustls::pki_types::ServerName::try_from(hostname.as_str())
                .map_err(|e| ImapError::connection(format!("Invalid hostname for TLS: {}", e)))?
                .to_owned(); // Convert to owned to avoid lifetime issues
            
            let tls_stream = connector.connect(domain, tcp_stream).await
                .map_err(|e| {
                    tracing::error!("TLS handshake failed with {}: {}", addr, e);
                    ImapError::connection(format!("TLS handshake failed: {}", e))
                })?;
            
            tracing::info!("TLS handshake with {} completed successfully", addr);
            
            // Split TLS stream
            let (read_half, write_half) = tokio::io::split(tls_stream);
            let reader = AsyncBufReader::new(read_half);
            let writer = AsyncBufWriter::new(write_half);
            
            SplitStream::Tls { reader, writer }
        } else {
            // Plain TCP connection
            let (read_half, write_half) = tcp_stream.into_split();
            let reader = AsyncBufReader::new(read_half);
            let writer = AsyncBufWriter::new(write_half);
            
            SplitStream::Plain { reader, writer }
        };
        
        self.stream = Some(split_stream);
        
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
        
        // Generate unique tag
        self.tag_counter += 1;
        let tag = format!("A{:04}", self.tag_counter);
        
        // Send command
        let full_command = format!("{} {}\r\n", tag, command);
        match self.stream.as_mut() {
            Some(SplitStream::Plain { writer, .. }) => {
                writer.write_all(full_command.as_bytes()).await
                    .map_err(|e| ImapError::connection(format!("Failed to send command: {}", e)))?;
                writer.flush().await
                    .map_err(|e| ImapError::connection(format!("Failed to flush command: {}", e)))?;
            }
            Some(SplitStream::Tls { writer, .. }) => {
                writer.write_all(full_command.as_bytes()).await
                    .map_err(|e| ImapError::connection(format!("Failed to send command: {}", e)))?;
                writer.flush().await
                    .map_err(|e| ImapError::connection(format!("Failed to flush command: {}", e)))?;
            }
            None => return Err(ImapError::invalid_state("No connection available")),
        }
        
        // Read response until we get the tagged response
        let mut responses = Vec::new();
        loop {
            let line = self.read_response().await?;
            
            // Check if this line contains a literal size indicator {size}
            if let Some(literal_size) = Self::extract_literal_size(&line) {
                tracing::debug!("Found literal of size {} in line: {}", literal_size, line);
                
                // Add the line with literal indicator
                responses.push(line.clone());
                
                // Read the literal data
                let literal_data = self.read_literal(literal_size).await?;
                
                // Convert literal data to string (assuming UTF-8 for now)
                let literal_string = String::from_utf8_lossy(&literal_data);
                responses.push(literal_string.to_string());
                
                tracing::debug!("Read literal content, length: {} chars", literal_string.len());
            } else {
                responses.push(line.clone());
            }
            
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
    
    /// Read a single response line from the server (public for IDLE)
    pub async fn read_response(&mut self) -> ImapResult<String> {
        let mut line = String::new();
        let timeout_duration = Duration::from_secs(self.config.timeout_seconds);
        
        let read_result = match self.stream.as_mut() {
            Some(SplitStream::Plain { reader, .. }) => {
                timeout(timeout_duration, reader.read_line(&mut line)).await
            }
            Some(SplitStream::Tls { reader, .. }) => {
                timeout(timeout_duration, reader.read_line(&mut line)).await
            }
            None => return Err(ImapError::invalid_state("No connection available")),
        };
        
        read_result
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
    
    /// Read exact number of bytes from the server (for IMAP literals)
    pub async fn read_literal(&mut self, byte_count: usize) -> ImapResult<Vec<u8>> {
        let mut buffer = vec![0u8; byte_count];
        let timeout_duration = Duration::from_secs(self.config.timeout_seconds);
        
        let read_result = match self.stream.as_mut() {
            Some(SplitStream::Plain { reader, .. }) => {
                timeout(timeout_duration, reader.read_exact(&mut buffer)).await
            }
            Some(SplitStream::Tls { reader, .. }) => {
                timeout(timeout_duration, reader.read_exact(&mut buffer)).await
            }
            None => return Err(ImapError::invalid_state("No connection available")),
        };
        
        read_result
            .map_err(|_| ImapError::Timeout)?
            .map_err(|e| ImapError::connection(format!("Failed to read literal: {}", e)))?;
        
        Ok(buffer)
    }
    
    /// Extract literal size from a line containing {size}
    fn extract_literal_size(line: &str) -> Option<usize> {
        // Look for {size} pattern at the end of the line
        if let Some(start) = line.rfind('{') {
            if let Some(end) = line[start..].find('}') {
                let size_str = &line[start + 1..start + end];
                if let Ok(size) = size_str.parse::<usize>() {
                    return Some(size);
                }
            }
        }
        None
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
    
    /// Send raw data to the server (for continuation responses)
    pub async fn send_raw(&mut self, data: &str) -> ImapResult<()> {
        if self.state == ConnectionState::Disconnected {
            return Err(ImapError::invalid_state("Not connected"));
        }
        
        match self.stream.as_mut() {
            Some(SplitStream::Plain { writer, .. }) => {
                writer.write_all(data.as_bytes()).await
                    .map_err(|e| ImapError::connection(format!("Failed to send raw data: {}", e)))?;
                writer.flush().await
                    .map_err(|e| ImapError::connection(format!("Failed to flush raw data: {}", e)))?;
            }
            Some(SplitStream::Tls { writer, .. }) => {
                writer.write_all(data.as_bytes()).await
                    .map_err(|e| ImapError::connection(format!("Failed to send raw data: {}", e)))?;
                writer.flush().await
                    .map_err(|e| ImapError::connection(format!("Failed to flush raw data: {}", e)))?;
            }
            None => return Err(ImapError::invalid_state("No connection available")),
        }
        
        Ok(())
    }
    
    /// Send AUTHENTICATE command with continuation handling
    pub async fn send_authenticate(&mut self, mechanism: &str, auth_data: &str) -> ImapResult<String> {
        if self.state == ConnectionState::Disconnected {
            return Err(ImapError::invalid_state("Not connected"));
        }
        
        // Generate unique tag
        self.tag_counter += 1;
        let tag = format!("A{:04}", self.tag_counter);
        
        println!("DEBUG: send_authenticate - Starting AUTHENTICATE {} with tag {}", mechanism, tag);
        
        // Step 1: Send AUTHENTICATE command
        let auth_command = format!("{} AUTHENTICATE {}\r\n", tag, mechanism);
        println!("DEBUG: send_authenticate - Sending command: {}", auth_command.trim());
        match self.stream.as_mut() {
            Some(SplitStream::Plain { writer, .. }) => {
                writer.write_all(auth_command.as_bytes()).await
                    .map_err(|e| ImapError::connection(format!("Failed to send authenticate command: {}", e)))?;
                writer.flush().await
                    .map_err(|e| ImapError::connection(format!("Failed to flush authenticate command: {}", e)))?;
            }
            Some(SplitStream::Tls { writer, .. }) => {
                writer.write_all(auth_command.as_bytes()).await
                    .map_err(|e| ImapError::connection(format!("Failed to send authenticate command: {}", e)))?;
                writer.flush().await
                    .map_err(|e| ImapError::connection(format!("Failed to flush authenticate command: {}", e)))?;
            }
            None => return Err(ImapError::invalid_state("No connection available")),
        }
        
        let mut responses = Vec::new();
        
        // Step 2: Read continuation response
        println!("DEBUG: send_authenticate - Waiting for continuation response...");
        let continuation = self.read_response().await?;
        println!("DEBUG: send_authenticate - Got continuation: {}", continuation);
        responses.push(continuation.clone());
        
        if !continuation.starts_with("+ ") {
            return Err(ImapError::protocol(format!("Expected continuation response, got: {}", continuation)));
        }
        
        // Step 3: Send authentication data
        println!("DEBUG: send_authenticate - Sending auth data (length: {})", auth_data.len());
        let data_command = format!("{}\r\n", auth_data);
        self.send_raw(&data_command).await?;
        
        // Step 4: Read final tagged response
        println!("DEBUG: send_authenticate - Reading final response...");
        loop {
            let line = self.read_response().await?;
            println!("DEBUG: send_authenticate - Got response line: {}", line);
            responses.push(line.clone());
            
            if line.starts_with(&tag) {
                // This is our tagged response
                if line.starts_with(&format!("{} OK", tag)) {
                    println!("DEBUG: send_authenticate - Authentication successful!");
                    break;
                } else if line.starts_with(&format!("{} NO", tag)) {
                    return Err(ImapError::server(format!("Authentication failed: {}", line)));
                } else if line.starts_with(&format!("{} BAD", tag)) {
                    return Err(ImapError::protocol(format!("Bad authenticate command: {}", line)));
                }
            } else if line.starts_with("+ ") {
                // This is an additional continuation response - could be an error from Gmail
                let continuation_data = &line[2..]; // Remove "+ " prefix
                if !continuation_data.is_empty() {
                    // Try to decode base64 and see if it's an error
                    if let Ok(decoded_bytes) = base64::prelude::BASE64_STANDARD.decode(continuation_data) {
                        if let Ok(decoded_str) = String::from_utf8(decoded_bytes) {
                            println!("DEBUG: send_authenticate - Decoded continuation: {}", decoded_str);
                            // Check if this looks like a JSON error response
                            if decoded_str.contains("\"status\"") && decoded_str.contains("400") {
                                println!("DEBUG: send_authenticate - Gmail returned error in continuation: {}", decoded_str);
                                // Send empty line to complete the authentication attempt
                                self.send_raw("\r\n").await?;
                                // Continue to read the final tagged response
                                continue;
                            }
                        }
                    }
                }
            }
        }
        
        Ok(responses.join("\n"))
    }
}

impl Drop for ImapConnection {
    fn drop(&mut self) {
        // Clean up connection in destructor
        // Note: This is synchronous cleanup - in async context we'd need explicit disconnect
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