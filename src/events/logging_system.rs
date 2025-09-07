use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock};
use tracing::{error, info, warn};
use uuid::Uuid;

/// Centralized logging and error tracking system
pub struct LoggingSystem {
    /// Configuration for logging behavior
    config: LoggingConfig,
    /// Error tracking and categorization
    error_tracker: Arc<RwLock<ErrorTracker>>,
    /// Performance and audit logs
    audit_logger: Arc<RwLock<AuditLogger>>,
    /// System health logger
    health_logger: Arc<RwLock<HealthLogger>>,
    /// Real-time log streaming
    log_streams: Arc<Mutex<HashMap<Uuid, LogStream>>>,
}

/// Configuration for the logging system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Minimum log level to capture
    pub log_level: LogLevel,
    /// Maximum number of errors to keep in memory
    pub max_error_history: usize,
    /// Maximum number of audit log entries to keep
    pub max_audit_history: usize,
    /// Enable structured JSON logging
    pub structured_logging: bool,
    /// Enable error correlation tracking
    pub error_correlation: bool,
    /// Log rotation interval
    pub log_rotation_interval: Duration,
    /// Enable real-time log streaming
    pub enable_streaming: bool,
    /// Batch size for log writing
    pub batch_size: usize,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            log_level: LogLevel::Info,
            max_error_history: 10000,
            max_audit_history: 50000,
            structured_logging: true,
            error_correlation: true,
            log_rotation_interval: Duration::from_secs(3600), // 1 hour
            enable_streaming: true,
            batch_size: 100,
        }
    }
}

/// Log severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum LogLevel {
    Trace = 0,
    Debug = 1,
    Info = 2,
    Warn = 3,
    Error = 4,
    Critical = 5,
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogLevel::Trace => write!(f, "TRACE"),
            LogLevel::Debug => write!(f, "DEBUG"),
            LogLevel::Info => write!(f, "INFO"),
            LogLevel::Warn => write!(f, "WARN"),
            LogLevel::Error => write!(f, "ERROR"),
            LogLevel::Critical => write!(f, "CRITICAL"),
        }
    }
}

/// Error tracking and analysis system
#[derive(Debug)]
pub struct ErrorTracker {
    /// Recent errors categorized by type
    errors_by_category: HashMap<ErrorCategory, VecDeque<TrackedError>>,
    /// Error correlation mappings
    error_correlations: HashMap<String, Vec<TrackedError>>,
    /// Error frequency analysis
    error_frequencies: HashMap<ErrorCategory, ErrorFrequency>,
    /// Total error counters
    total_errors: u64,
    /// Critical error count
    critical_errors: u64,
}

/// Error categories for better analysis and handling
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorCategory {
    /// Network connectivity issues
    NetworkError,
    /// Authentication and authorization failures
    AuthenticationError,
    /// Database operation failures
    DatabaseError,
    /// Email protocol errors (IMAP, SMTP)
    EmailProtocolError,
    /// Calendar protocol errors (CalDAV)
    CalendarProtocolError,
    /// File system and I/O errors
    FileSystemError,
    /// Configuration and parsing errors
    ConfigurationError,
    /// Memory and resource exhaustion
    ResourceError,
    /// Application logic errors
    ApplicationError,
    /// External service failures
    ExternalServiceError,
    /// Validation and input errors
    ValidationError,
    /// Unexpected system errors
    SystemError,
}

/// Detailed error tracking information
#[derive(Debug, Clone)]
pub struct TrackedError {
    /// Unique error identifier
    pub id: Uuid,
    /// When the error occurred
    pub timestamp: Instant,
    /// Error category
    pub category: ErrorCategory,
    /// Error severity level
    pub level: LogLevel,
    /// Human-readable error message
    pub message: String,
    /// Error details and context
    pub details: HashMap<String, String>,
    /// Stack trace if available
    pub stack_trace: Option<String>,
    /// Operation that caused the error
    pub operation: Option<String>,
    /// User or system context
    pub context: Option<String>,
    /// Related errors (correlation ID)
    pub correlation_id: Option<String>,
    /// Recovery attempts made
    pub recovery_attempts: u32,
    /// Whether error was recovered
    pub recovered: bool,
}

/// Error frequency analysis
#[derive(Debug, Clone)]
pub struct ErrorFrequency {
    /// Total occurrences
    pub total_count: u64,
    /// Recent occurrences (last hour)
    pub recent_count: u64,
    /// Error rate (errors per minute)
    pub error_rate: f64,
    /// First occurrence timestamp
    pub first_seen: Instant,
    /// Last occurrence timestamp
    pub last_seen: Instant,
    /// Average time between occurrences
    pub avg_interval: Duration,
}

/// Audit logging for important operations
#[derive(Debug)]
pub struct AuditLogger {
    /// Audit log entries
    audit_logs: VecDeque<AuditLogEntry>,
    /// Audit log statistics
    statistics: AuditStatistics,
}

/// Individual audit log entry
#[derive(Debug, Clone)]
pub struct AuditLogEntry {
    /// Entry timestamp
    pub timestamp: Instant,
    /// Operation that was performed
    pub operation: String,
    /// User or system performing operation
    pub actor: String,
    /// Resource being operated on
    pub resource: Option<String>,
    /// Operation result status
    pub status: AuditStatus,
    /// Additional context information
    pub metadata: HashMap<String, String>,
    /// Duration of operation
    pub duration: Option<Duration>,
    /// Request/correlation ID
    pub correlation_id: Option<String>,
}

/// Audit operation status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuditStatus {
    Success,
    Failed,
    Partial,
    Cancelled,
}

/// Audit logging statistics
#[derive(Debug, Clone, Default, Serialize)]
pub struct AuditStatistics {
    pub total_operations: u64,
    pub successful_operations: u64,
    pub failed_operations: u64,
    pub average_operation_duration: Duration,
    pub operations_by_type: HashMap<String, u64>,
}

/// System health logging
#[derive(Debug)]
pub struct HealthLogger {
    /// Health status history
    health_history: VecDeque<HealthLogEntry>,
    /// Current system status
    current_status: SystemHealthStatus,
    /// Health metrics
    health_metrics: HealthMetrics,
}

/// Health log entry
#[derive(Debug, Clone)]
pub struct HealthLogEntry {
    pub timestamp: Instant,
    pub status: SystemHealthStatus,
    pub metrics: HealthMetrics,
    pub issues: Vec<String>,
    pub recovery_actions: Vec<String>,
}

/// System health status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SystemHealthStatus {
    Healthy,
    Warning,
    Critical,
    Down,
}

/// Health metrics snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthMetrics {
    pub memory_usage_mb: f64,
    pub cpu_usage_percent: f64,
    pub disk_usage_percent: f64,
    pub network_latency_ms: f64,
    pub active_connections: u32,
    pub error_rate: f64,
    pub throughput: f64,
}

impl Default for HealthMetrics {
    fn default() -> Self {
        Self {
            memory_usage_mb: 0.0,
            cpu_usage_percent: 0.0,
            disk_usage_percent: 0.0,
            network_latency_ms: 0.0,
            active_connections: 0,
            error_rate: 0.0,
            throughput: 0.0,
        }
    }
}

/// Real-time log streaming
#[derive(Debug)]
pub struct LogStream {
    /// Stream identifier
    pub id: Uuid,
    /// Log level filter
    pub filter_level: LogLevel,
    /// Category filters
    pub category_filters: Vec<ErrorCategory>,
    /// Recent log entries
    pub recent_entries: VecDeque<LogEntry>,
    /// Maximum entries to keep
    pub max_entries: usize,
    /// Stream creation time
    pub created_at: Instant,
}

/// Generic log entry for streaming
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: Instant,
    pub level: LogLevel,
    pub category: Option<ErrorCategory>,
    pub message: String,
    pub details: HashMap<String, String>,
}

impl LoggingSystem {
    pub fn new() -> Self {
        Self {
            config: LoggingConfig::default(),
            error_tracker: Arc::new(RwLock::new(ErrorTracker::new())),
            audit_logger: Arc::new(RwLock::new(AuditLogger::new())),
            health_logger: Arc::new(RwLock::new(HealthLogger::new())),
            log_streams: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn with_config(config: LoggingConfig) -> Self {
        Self {
            config,
            error_tracker: Arc::new(RwLock::new(ErrorTracker::new())),
            audit_logger: Arc::new(RwLock::new(AuditLogger::new())),
            health_logger: Arc::new(RwLock::new(HealthLogger::new())),
            log_streams: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Log an error with full context and tracking
    pub async fn log_error(
        &self,
        category: ErrorCategory,
        level: LogLevel,
        message: String,
        operation: Option<String>,
        context: Option<String>,
        details: HashMap<String, String>,
        correlation_id: Option<String>,
    ) -> Uuid {
        let error = TrackedError {
            id: Uuid::new_v4(),
            timestamp: Instant::now(),
            category: category.clone(),
            level,
            message: message.clone(),
            details: details.clone(),
            stack_trace: None, // Could be populated with backtrace in the future
            operation: operation.clone(),
            context,
            correlation_id: correlation_id.clone(),
            recovery_attempts: 0,
            recovered: false,
        };

        // Track the error
        let error_id = error.id;
        let mut tracker = self.error_tracker.write().await;
        tracker.track_error(error.clone()).await;
        drop(tracker);

        // Log to structured logging
        match level {
            LogLevel::Critical => error!(
                category = ?category,
                error_id = ?error_id,
                operation = ?operation,
                correlation_id = ?correlation_id,
                details = ?details,
                "Critical error: {}", message
            ),
            LogLevel::Error => error!(
                category = ?category,
                error_id = ?error_id,
                operation = ?operation,
                correlation_id = ?correlation_id,
                details = ?details,
                "Error: {}", message
            ),
            LogLevel::Warn => warn!(
                category = ?category,
                error_id = ?error_id,
                operation = ?operation,
                correlation_id = ?correlation_id,
                details = ?details,
                "Warning: {}", message
            ),
            _ => info!(
                category = ?category,
                error_id = ?error_id,
                operation = ?operation,
                correlation_id = ?correlation_id,
                details = ?details,
                "Info: {}", message
            ),
        }

        // Stream to active log streams
        if self.config.enable_streaming {
            self.stream_log_entry(LogEntry {
                timestamp: error.timestamp,
                level,
                category: Some(category),
                message,
                details,
            })
            .await;
        }

        error_id
    }

    /// Log audit trail for important operations
    pub async fn log_audit(
        &self,
        operation: String,
        actor: String,
        resource: Option<String>,
        status: AuditStatus,
        duration: Option<Duration>,
        metadata: HashMap<String, String>,
        correlation_id: Option<String>,
    ) {
        let entry = AuditLogEntry {
            timestamp: Instant::now(),
            operation: operation.clone(),
            actor: actor.clone(),
            resource: resource.clone(),
            status,
            metadata: metadata.clone(),
            duration,
            correlation_id: correlation_id.clone(),
        };

        let mut audit_logger = self.audit_logger.write().await;
        audit_logger.log_entry(entry);
        drop(audit_logger);

        info!(
            operation = %operation,
            actor = %actor,
            resource = ?resource,
            status = ?status,
            duration = ?duration,
            correlation_id = ?correlation_id,
            metadata = ?metadata,
            "Audit log: {} performed by {} with status {:?}", operation, actor, status
        );
    }

    /// Log system health status
    pub async fn log_health(
        &self,
        status: SystemHealthStatus,
        metrics: HealthMetrics,
        issues: Vec<String>,
        recovery_actions: Vec<String>,
    ) {
        let entry = HealthLogEntry {
            timestamp: Instant::now(),
            status,
            metrics: metrics.clone(),
            issues: issues.clone(),
            recovery_actions: recovery_actions.clone(),
        };

        let mut health_logger = self.health_logger.write().await;
        health_logger.log_health(entry);
        drop(health_logger);

        match status {
            SystemHealthStatus::Critical | SystemHealthStatus::Down => error!(
                status = ?status,
                memory_mb = metrics.memory_usage_mb,
                cpu_percent = metrics.cpu_usage_percent,
                error_rate = metrics.error_rate,
                issues = ?issues,
                recovery_actions = ?recovery_actions,
                "System health critical"
            ),
            SystemHealthStatus::Warning => warn!(
                status = ?status,
                memory_mb = metrics.memory_usage_mb,
                cpu_percent = metrics.cpu_usage_percent,
                error_rate = metrics.error_rate,
                issues = ?issues,
                "System health warning"
            ),
            SystemHealthStatus::Healthy => info!(
                status = ?status,
                memory_mb = metrics.memory_usage_mb,
                cpu_percent = metrics.cpu_usage_percent,
                error_rate = metrics.error_rate,
                "System health check"
            ),
        }
    }

    /// Create a new real-time log stream
    pub async fn create_log_stream(
        &self,
        filter_level: LogLevel,
        category_filters: Vec<ErrorCategory>,
        max_entries: usize,
    ) -> Uuid {
        let stream_id = Uuid::new_v4();
        let stream = LogStream {
            id: stream_id,
            filter_level,
            category_filters,
            recent_entries: VecDeque::new(),
            max_entries,
            created_at: Instant::now(),
        };

        let mut streams = self.log_streams.lock().await;
        streams.insert(stream_id, stream);

        stream_id
    }

    /// Get recent entries from a log stream
    pub async fn get_stream_entries(&self, stream_id: Uuid) -> Option<Vec<LogEntry>> {
        let streams = self.log_streams.lock().await;
        streams
            .get(&stream_id)
            .map(|stream| stream.recent_entries.iter().cloned().collect())
    }

    /// Remove a log stream
    pub async fn remove_stream(&self, stream_id: Uuid) -> bool {
        let mut streams = self.log_streams.lock().await;
        streams.remove(&stream_id).is_some()
    }

    /// Get error statistics and analysis
    pub async fn get_error_analysis(&self) -> ErrorAnalysis {
        let tracker = self.error_tracker.read().await;
        tracker.get_analysis().await
    }

    /// Get audit log summary
    pub async fn get_audit_summary(&self) -> AuditStatistics {
        let audit_logger = self.audit_logger.read().await;
        audit_logger.statistics.clone()
    }

    /// Get current system health status
    pub async fn get_health_status(&self) -> SystemHealthStatus {
        let health_logger = self.health_logger.read().await;
        health_logger.current_status
    }

    /// Stream log entry to all active streams
    async fn stream_log_entry(&self, entry: LogEntry) {
        if !self.config.enable_streaming {
            return;
        }

        let mut streams = self.log_streams.lock().await;
        for stream in streams.values_mut() {
            // Check if entry matches stream filters
            if entry.level >= stream.filter_level {
                if stream.category_filters.is_empty()
                    || entry
                        .category
                        .as_ref()
                        .map_or(true, |cat| stream.category_filters.contains(cat))
                {
                    stream.recent_entries.push_back(entry.clone());

                    // Maintain max entries limit
                    while stream.recent_entries.len() > stream.max_entries {
                        stream.recent_entries.pop_front();
                    }
                }
            }
        }
    }
}

impl ErrorTracker {
    pub fn new() -> Self {
        Self {
            errors_by_category: HashMap::new(),
            error_correlations: HashMap::new(),
            error_frequencies: HashMap::new(),
            total_errors: 0,
            critical_errors: 0,
        }
    }

    pub async fn track_error(&mut self, error: TrackedError) {
        self.total_errors += 1;

        if error.level >= LogLevel::Critical {
            self.critical_errors += 1;
        }

        // Track by category
        let category_errors = self
            .errors_by_category
            .entry(error.category.clone())
            .or_insert_with(VecDeque::new);
        category_errors.push_back(error.clone());

        // Maintain history limits
        while category_errors.len() > 1000 {
            category_errors.pop_front();
        }

        // Update frequency statistics
        let frequency = self
            .error_frequencies
            .entry(error.category.clone())
            .or_insert_with(|| ErrorFrequency {
                total_count: 0,
                recent_count: 0,
                error_rate: 0.0,
                first_seen: error.timestamp,
                last_seen: error.timestamp,
                avg_interval: Duration::from_secs(0),
            });

        frequency.total_count += 1;
        frequency.last_seen = error.timestamp;

        // Track correlations
        if let Some(correlation_id) = &error.correlation_id {
            let correlated_errors = self
                .error_correlations
                .entry(correlation_id.clone())
                .or_insert_with(Vec::new);
            correlated_errors.push(error);
        }
    }

    pub async fn get_analysis(&self) -> ErrorAnalysis {
        ErrorAnalysis {
            total_errors: self.total_errors,
            critical_errors: self.critical_errors,
            categories: self.error_frequencies.clone(),
            recent_errors: self.get_recent_errors(Duration::from_secs(3600)), // Last hour
        }
    }

    fn get_recent_errors(&self, duration: Duration) -> Vec<TrackedError> {
        let cutoff = Instant::now() - duration;
        let mut recent = Vec::new();

        for errors in self.errors_by_category.values() {
            for error in errors {
                if error.timestamp >= cutoff {
                    recent.push(error.clone());
                }
            }
        }

        recent.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        recent
    }
}

impl AuditLogger {
    pub fn new() -> Self {
        Self {
            audit_logs: VecDeque::new(),
            statistics: AuditStatistics::default(),
        }
    }

    pub fn log_entry(&mut self, entry: AuditLogEntry) {
        self.statistics.total_operations += 1;

        match entry.status {
            AuditStatus::Success => self.statistics.successful_operations += 1,
            AuditStatus::Failed => self.statistics.failed_operations += 1,
            _ => {}
        }

        // Update operation type statistics
        *self
            .statistics
            .operations_by_type
            .entry(entry.operation.clone())
            .or_insert(0) += 1;

        // Update average duration
        if let Some(duration) = entry.duration {
            let current_avg = self.statistics.average_operation_duration;
            let total_ops = self.statistics.total_operations;
            self.statistics.average_operation_duration =
                (current_avg * (total_ops - 1) as u32 + duration) / total_ops as u32;
        }

        self.audit_logs.push_back(entry);

        // Maintain size limits
        while self.audit_logs.len() > 50000 {
            self.audit_logs.pop_front();
        }
    }
}

impl HealthLogger {
    pub fn new() -> Self {
        Self {
            health_history: VecDeque::new(),
            current_status: SystemHealthStatus::Healthy,
            health_metrics: HealthMetrics::default(),
        }
    }

    pub fn log_health(&mut self, entry: HealthLogEntry) {
        self.current_status = entry.status;
        self.health_metrics = entry.metrics.clone();

        self.health_history.push_back(entry);

        // Keep last 10000 health entries
        while self.health_history.len() > 10000 {
            self.health_history.pop_front();
        }
    }
}

/// Error analysis results
#[derive(Debug, Clone)]
pub struct ErrorAnalysis {
    pub total_errors: u64,
    pub critical_errors: u64,
    pub categories: HashMap<ErrorCategory, ErrorFrequency>,
    pub recent_errors: Vec<TrackedError>,
}

impl Default for ErrorTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for AuditLogger {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for HealthLogger {
    fn default() -> Self {
        Self::new()
    }
}
