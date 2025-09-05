//! Remote session support for SSH and Mosh connections

use super::{MultiplexerResult}; // MultiplexerError
use serde::{Deserialize, Serialize};
// use std::collections::HashMap;

/// Remote session manager
#[allow(dead_code)]
pub struct RemoteSession {
    session_type: RemoteSessionType,
    connection_info: ConnectionInfo,
    optimizations: RemoteOptimizations,
}

/// Types of remote sessions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RemoteSessionType {
    SSH,
    Mosh,
    Local,
}

/// Connection information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionInfo {
    pub hostname: Option<String>,
    pub username: Option<String>,
    pub port: Option<u16>,
    pub latency_ms: Option<u64>,
    pub bandwidth_kbps: Option<u64>,
}

/// Optimizations for remote sessions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteOptimizations {
    pub reduce_updates: bool,
    pub compress_data: bool,
    pub buffer_output: bool,
    pub limit_animations: bool,
}

/// SSH-specific session handling
pub struct SSHSession {
    connection_info: ConnectionInfo,
}

/// Mosh-specific session handling  
pub struct MoshSession {
    connection_info: ConnectionInfo,
}

impl RemoteSession {
    pub fn detect() -> MultiplexerResult<Self> {
        let session_type = Self::detect_session_type()?;
        let connection_info = Self::gather_connection_info(&session_type)?;
        
        Ok(Self {
            session_type: session_type.clone(),
            connection_info,
            optimizations: RemoteOptimizations::for_session_type(&session_type),
        })
    }

    fn detect_session_type() -> MultiplexerResult<RemoteSessionType> {
        if std::env::var("SSH_CONNECTION").is_ok() {
            Ok(RemoteSessionType::SSH)
        } else if std::env::var("MOSH_CONNECTION").is_ok() {
            Ok(RemoteSessionType::Mosh)
        } else {
            Ok(RemoteSessionType::Local)
        }
    }

    fn gather_connection_info(session_type: &RemoteSessionType) -> MultiplexerResult<ConnectionInfo> {
        match session_type {
            RemoteSessionType::SSH => {
                let ssh_connection = std::env::var("SSH_CONNECTION").unwrap_or_default();
                let parts: Vec<&str> = ssh_connection.split_whitespace().collect();
                
                Ok(ConnectionInfo {
                    hostname: std::env::var("SSH_CLIENT").ok(),
                    username: std::env::var("USER").ok(),
                    port: if parts.len() >= 4 { parts[3].parse().ok() } else { None },
                    latency_ms: None, // Would need to measure
                    bandwidth_kbps: None, // Would need to measure
                })
            }
            RemoteSessionType::Mosh => {
                Ok(ConnectionInfo {
                    hostname: None, // Mosh doesn't expose this easily
                    username: std::env::var("USER").ok(),
                    port: None,
                    latency_ms: None,
                    bandwidth_kbps: None,
                })
            }
            RemoteSessionType::Local => {
                Ok(ConnectionInfo {
                    hostname: None,
                    username: std::env::var("USER").ok(),
                    port: None,
                    latency_ms: Some(0),
                    bandwidth_kbps: None,
                })
            }
        }
    }

    pub fn is_remote(&self) -> bool {
        !matches!(self.session_type, RemoteSessionType::Local)
    }

    pub fn should_optimize(&self) -> bool {
        self.is_remote()
    }

    pub fn optimizations(&self) -> &RemoteOptimizations {
        &self.optimizations
    }
}

impl RemoteOptimizations {
    fn for_session_type(session_type: &RemoteSessionType) -> Self {
        match session_type {
            RemoteSessionType::SSH => Self {
                reduce_updates: true,
                compress_data: true,
                buffer_output: true,
                limit_animations: true,
            },
            RemoteSessionType::Mosh => Self {
                reduce_updates: false, // Mosh handles this
                compress_data: false,  // Mosh handles this
                buffer_output: false,  // Mosh handles this
                limit_animations: true,
            },
            RemoteSessionType::Local => Self {
                reduce_updates: false,
                compress_data: false,
                buffer_output: false,
                limit_animations: false,
            },
        }
    }
}

impl SSHSession {
    pub fn new() -> MultiplexerResult<Self> {
        Ok(Self {
            connection_info: ConnectionInfo {
                hostname: None,
                username: None,
                port: None,
                latency_ms: None,
                bandwidth_kbps: None,
            },
        })
    }
}

impl MoshSession {
    pub fn new() -> MultiplexerResult<Self> {
        Ok(Self {
            connection_info: ConnectionInfo {
                hostname: None,
                username: None,
                port: None,
                latency_ms: None,
                bandwidth_kbps: None,
            },
        })
    }
}