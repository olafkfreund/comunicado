//! Streaming response system for real-time AI operation feedback

use crate::ai::{AIResult, AIError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info};
use uuid::Uuid;

/// Streaming response chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunk {
    /// Operation ID this chunk belongs to
    pub operation_id: Uuid,
    /// Chunk sequence number
    pub sequence: usize,
    /// Chunk content
    pub content: String,
    /// Whether this is the final chunk
    pub is_final: bool,
    /// Timestamp when chunk was generated
    pub timestamp: std::time::SystemTime,
    /// Chunk metadata
    pub metadata: HashMap<String, String>,
}

/// Streaming session for an AI operation
#[derive(Debug, Clone)]
pub struct StreamingSession {
    /// Unique session ID
    pub id: Uuid,
    /// Operation ID this session belongs to
    pub operation_id: Uuid,
    /// When the session started
    pub started_at: Instant,
    /// Total chunks received
    pub chunks_received: usize,
    /// Whether the stream is complete
    pub is_complete: bool,
    /// Accumulated content
    pub accumulated_content: String,
    /// Stream metadata
    pub metadata: HashMap<String, String>,
}

impl StreamingSession {
    /// Create a new streaming session
    pub fn new(operation_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            operation_id,
            started_at: Instant::now(),
            chunks_received: 0,
            is_complete: false,
            accumulated_content: String::new(),
            metadata: HashMap::new(),
        }
    }

    /// Add a chunk to the session
    pub fn add_chunk(&mut self, chunk: &StreamChunk) {
        self.chunks_received += 1;
        self.accumulated_content.push_str(&chunk.content);
        self.is_complete = chunk.is_final;
        
        // Merge chunk metadata
        for (key, value) in &chunk.metadata {
            self.metadata.insert(key.clone(), value.clone());
        }
    }

    /// Get session duration
    pub fn duration(&self) -> Duration {
        self.started_at.elapsed()
    }

    /// Get streaming rate (chunks per second)
    pub fn chunks_per_second(&self) -> f64 {
        if self.duration().as_secs() > 0 {
            self.chunks_received as f64 / self.duration().as_secs_f64()
        } else {
            0.0
        }
    }

    /// Get average chunk size
    pub fn average_chunk_size(&self) -> usize {
        if self.chunks_received > 0 {
            self.accumulated_content.len() / self.chunks_received
        } else {
            0
        }
    }
}

/// Configuration for streaming responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingConfig {
    /// Maximum number of active streams
    pub max_active_streams: usize,
    /// Buffer size for stream channels
    pub buffer_size: usize,
    /// Timeout for inactive streams
    pub stream_timeout: Duration,
    /// Maximum chunk size
    pub max_chunk_size: usize,
    /// Minimum interval between chunks
    pub min_chunk_interval: Duration,
    /// Enable chunk compression
    pub enable_compression: bool,
}

impl Default for StreamingConfig {
    fn default() -> Self {
        Self {
            max_active_streams: 10,
            buffer_size: 1000,
            stream_timeout: Duration::from_secs(300), // 5 minutes
            max_chunk_size: 8192, // 8KB
            min_chunk_interval: Duration::from_millis(10),
            enable_compression: false,
        }
    }
}

/// Streaming manager for AI operations
pub struct AIStreamingManager {
    /// Configuration
    config: Arc<RwLock<StreamingConfig>>,
    /// Active streaming sessions
    sessions: Arc<RwLock<HashMap<Uuid, StreamingSession>>>,
    /// Stream senders for each operation
    stream_senders: Arc<RwLock<HashMap<Uuid, mpsc::UnboundedSender<StreamChunk>>>>,
    /// Statistics
    stats: Arc<RwLock<StreamingStats>>,
}

/// Streaming statistics
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct StreamingStats {
    /// Total streams created
    pub total_streams: usize,
    /// Currently active streams
    pub active_streams: usize,
    /// Total chunks processed
    pub total_chunks: usize,
    /// Total bytes streamed
    pub total_bytes: usize,
    /// Average stream duration
    pub avg_stream_duration: Duration,
    /// Average chunks per stream
    pub avg_chunks_per_stream: f64,
    /// Streams by status
    pub streams_by_status: HashMap<String, usize>,
}

impl AIStreamingManager {
    /// Create a new streaming manager
    pub fn new(config: StreamingConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            sessions: Arc::new(RwLock::new(HashMap::new())),
            stream_senders: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(StreamingStats::default())),
        }
    }

    /// Start a new streaming session for an operation
    pub async fn start_stream(&self, operation_id: Uuid) -> AIResult<mpsc::UnboundedReceiver<StreamChunk>> {
        let config = self.config.read().await;
        let mut sessions = self.sessions.write().await;
        let mut senders = self.stream_senders.write().await;

        // Check if we've reached the maximum number of active streams
        if sessions.len() >= config.max_active_streams {
            return Err(AIError::internal_error("Maximum number of active streams reached"));
        }

        // Create new session
        let session = StreamingSession::new(operation_id);
        let session_id = session.id;

        // Create channel for streaming
        let (tx, rx) = mpsc::unbounded_channel();

        // Store session and sender
        sessions.insert(session_id, session);
        senders.insert(operation_id, tx);

        // Update statistics
        let mut stats = self.stats.write().await;
        stats.total_streams += 1;
        stats.active_streams = sessions.len();

        info!("Started streaming session {} for operation {}", session_id, operation_id);

        Ok(rx)
    }

    /// Send a chunk to a streaming session
    pub async fn send_chunk(&self, chunk: StreamChunk) -> AIResult<()> {
        let operation_id = chunk.operation_id;
        
        // Validate chunk size
        {
            let config = self.config.read().await;
            if chunk.content.len() > config.max_chunk_size {
                return Err(AIError::internal_error("Chunk size exceeds maximum"));
            }
        }

        // Find and update session
        {
            let mut sessions = self.sessions.write().await;
            if let Some(session) = sessions.values_mut().find(|s| s.operation_id == operation_id) {
                session.add_chunk(&chunk);
            }
        }

        // Send chunk through channel
        {
            let senders = self.stream_senders.read().await;
            if let Some(sender) = senders.get(&operation_id) {
                if sender.send(chunk.clone()).is_err() {
                    error!("Failed to send chunk for operation {}", operation_id);
                    return Err(AIError::internal_error("Failed to send stream chunk"));
                }
            } else {
                return Err(AIError::internal_error("No active stream for operation"));
            }
        }

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.total_chunks += 1;
            stats.total_bytes += chunk.content.len();
        }

        // If this is the final chunk, clean up the stream
        if chunk.is_final {
            self.end_stream(operation_id).await?;
        }

        debug!("Sent chunk {} for operation {}", chunk.sequence, operation_id);

        Ok(())
    }

    /// End a streaming session
    pub async fn end_stream(&self, operation_id: Uuid) -> AIResult<StreamingSession> {
        let mut sessions = self.sessions.write().await;
        let mut senders = self.stream_senders.write().await;

        // Remove sender first to close the channel
        senders.remove(&operation_id);

        // Find and remove session
        let session = sessions
            .values()
            .find(|s| s.operation_id == operation_id)
            .cloned();

        if let Some(session) = session {
            // Remove by session ID
            sessions.remove(&session.id);

            // Update statistics
            {
                let mut stats = self.stats.write().await;
                stats.active_streams = sessions.len();
                
                // Update average duration
                let total_completed = stats.total_streams - stats.active_streams;
                if total_completed > 1 {
                    stats.avg_stream_duration = Duration::from_millis(
                        ((stats.avg_stream_duration.as_millis() * (total_completed - 1) as u128
                         + session.duration().as_millis()) / total_completed as u128)
                         .try_into().unwrap_or(0)
                    );
                } else {
                    stats.avg_stream_duration = session.duration();
                }

                // Update average chunks per stream
                if total_completed > 0 {
                    stats.avg_chunks_per_stream = stats.total_chunks as f64 / total_completed as f64;
                }
            }

            info!("Ended streaming session {} for operation {} (duration: {:?}, chunks: {})", 
                  session.id, operation_id, session.duration(), session.chunks_received);

            Ok(session)
        } else {
            Err(AIError::internal_error("Stream session not found"))
        }
    }

    /// Get streaming session information
    pub async fn get_session(&self, operation_id: Uuid) -> Option<StreamingSession> {
        let sessions = self.sessions.read().await;
        sessions.values().find(|s| s.operation_id == operation_id).cloned()
    }

    /// Get all active sessions
    pub async fn get_active_sessions(&self) -> Vec<StreamingSession> {
        let sessions = self.sessions.read().await;
        sessions.values().cloned().collect()
    }

    /// Get streaming statistics
    pub async fn get_stats(&self) -> StreamingStats {
        let stats = self.stats.read().await;
        stats.clone()
    }

    /// Cleanup expired streams
    pub async fn cleanup_expired_streams(&self) -> usize {
        let config = self.config.read().await;
        let timeout = config.stream_timeout;
        drop(config);

        let mut expired_operations = Vec::new();

        // Find expired sessions
        {
            let sessions = self.sessions.read().await;
            for session in sessions.values() {
                if session.duration() > timeout && !session.is_complete {
                    expired_operations.push(session.operation_id);
                }
            }
        }

        // Clean up expired sessions
        let mut cleaned_count = 0;
        for operation_id in expired_operations {
            if self.end_stream(operation_id).await.is_ok() {
                cleaned_count += 1;
            }
        }

        if cleaned_count > 0 {
            info!("Cleaned up {} expired streaming sessions", cleaned_count);
        }

        cleaned_count
    }

    /// Start periodic cleanup task
    pub async fn start_cleanup_task(&self) {
        let manager = Arc::new(self.clone());
        
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(60)).await; // Check every minute
                manager.cleanup_expired_streams().await;
            }
        });
    }

    /// Create a streaming chunk
    pub fn create_chunk(
        operation_id: Uuid,
        sequence: usize,
        content: String,
        is_final: bool,
    ) -> StreamChunk {
        StreamChunk {
            operation_id,
            sequence,
            content,
            is_final,
            timestamp: std::time::SystemTime::now(),
            metadata: HashMap::new(),
        }
    }

    /// Create a streaming chunk with metadata
    pub fn create_chunk_with_metadata(
        operation_id: Uuid,
        sequence: usize,
        content: String,
        is_final: bool,
        metadata: HashMap<String, String>,
    ) -> StreamChunk {
        StreamChunk {
            operation_id,
            sequence,
            content,
            is_final,
            timestamp: std::time::SystemTime::now(),
            metadata,
        }
    }
}

impl Clone for AIStreamingManager {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            sessions: self.sessions.clone(),
            stream_senders: self.stream_senders.clone(),
            stats: self.stats.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::timeout;

    #[tokio::test]
    async fn test_streaming_session_creation() {
        let operation_id = Uuid::new_v4();
        let session = StreamingSession::new(operation_id);
        
        assert_eq!(session.operation_id, operation_id);
        assert_eq!(session.chunks_received, 0);
        assert!(!session.is_complete);
        assert!(session.accumulated_content.is_empty());
    }

    #[tokio::test]
    async fn test_chunk_addition() {
        let operation_id = Uuid::new_v4();
        let mut session = StreamingSession::new(operation_id);
        
        let chunk = StreamChunk {
            operation_id,
            sequence: 1,
            content: "Hello ".to_string(),
            is_final: false,
            timestamp: std::time::SystemTime::now(),
            metadata: HashMap::new(),
        };
        
        session.add_chunk(&chunk);
        
        assert_eq!(session.chunks_received, 1);
        assert_eq!(session.accumulated_content, "Hello ");
        assert!(!session.is_complete);
    }

    #[tokio::test]
    async fn test_streaming_manager() {
        let config = StreamingConfig::default();
        let manager = AIStreamingManager::new(config);
        let operation_id = Uuid::new_v4();

        // Start stream
        let mut rx = manager.start_stream(operation_id).await.unwrap();

        // Send chunk
        let chunk = AIStreamingManager::create_chunk(
            operation_id,
            1,
            "Test content".to_string(),
            false,
        );
        
        manager.send_chunk(chunk).await.unwrap();

        // Receive chunk
        let received_chunk = timeout(Duration::from_millis(100), rx.recv())
            .await
            .unwrap()
            .unwrap();
        
        assert_eq!(received_chunk.content, "Test content");
        assert_eq!(received_chunk.sequence, 1);
        assert!(!received_chunk.is_final);

        // Send final chunk
        let final_chunk = AIStreamingManager::create_chunk(
            operation_id,
            2,
            " Final".to_string(),
            true,
        );
        
        manager.send_chunk(final_chunk).await.unwrap();

        // Receive final chunk
        let final_received = timeout(Duration::from_millis(100), rx.recv())
            .await
            .unwrap()
            .unwrap();
        
        assert_eq!(final_received.content, " Final");
        assert!(final_received.is_final);

        // Stream should be ended
        assert!(manager.get_session(operation_id).await.is_none());
    }

    #[tokio::test]
    async fn test_streaming_statistics() {
        let config = StreamingConfig::default();
        let manager = AIStreamingManager::new(config);
        let operation_id = Uuid::new_v4();

        let initial_stats = manager.get_stats().await;
        assert_eq!(initial_stats.total_streams, 0);
        assert_eq!(initial_stats.active_streams, 0);

        // Start stream
        let _rx = manager.start_stream(operation_id).await.unwrap();

        let stats_after_start = manager.get_stats().await;
        assert_eq!(stats_after_start.total_streams, 1);
        assert_eq!(stats_after_start.active_streams, 1);

        // Send chunk
        let chunk = AIStreamingManager::create_chunk(
            operation_id,
            1,
            "Test".to_string(),
            true, // Final chunk
        );
        
        manager.send_chunk(chunk).await.unwrap();

        let final_stats = manager.get_stats().await;
        assert_eq!(final_stats.total_chunks, 1);
        assert_eq!(final_stats.total_bytes, 4); // "Test" = 4 bytes
        assert_eq!(final_stats.active_streams, 0); // Stream ended
    }
}