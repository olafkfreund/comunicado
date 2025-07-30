use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq)]
pub enum StartupPhase {
    Database { timeout: Duration, status: PhaseStatus },
    ImapManager { timeout: Duration, status: PhaseStatus },
    AccountSetup { timeout: Duration, status: PhaseStatus },
    Services { timeout: Duration, status: PhaseStatus },
}

#[derive(Debug, Clone, PartialEq)]
pub enum PhaseStatus {
    Pending,
    InProgress { started_at: Instant },
    Completed { duration: Duration },
    Failed { error: String },
    TimedOut { duration: Duration },
}

#[derive(Debug, Clone, PartialEq)]
pub struct StartupError {
    pub phase: String,
    pub error: String,
    pub is_critical: bool,
}

impl StartupPhase {
    pub fn name(&self) -> &'static str {
        match self {
            StartupPhase::Database { .. } => "Database",
            StartupPhase::ImapManager { .. } => "IMAP Manager",
            StartupPhase::AccountSetup { .. } => "Account Setup",
            StartupPhase::Services { .. } => "Services",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            StartupPhase::Database { .. } => "Initializing database connection and schema",
            StartupPhase::ImapManager { .. } => "Setting up IMAP account management",
            StartupPhase::AccountSetup { .. } => "Checking accounts and running setup wizard",
            StartupPhase::Services { .. } => "Initializing core application services",
        }
    }

    pub fn status(&self) -> &PhaseStatus {
        match self {
            StartupPhase::Database { status, .. } => status,
            StartupPhase::ImapManager { status, .. } => status,
            StartupPhase::AccountSetup { status, .. } => status,
            StartupPhase::Services { status, .. } => status,
        }
    }

    pub fn status_mut(&mut self) -> &mut PhaseStatus {
        match self {
            StartupPhase::Database { status, .. } => status,
            StartupPhase::ImapManager { status, .. } => status,
            StartupPhase::AccountSetup { status, .. } => status,
            StartupPhase::Services { status, .. } => status,
        }
    }

    pub fn timeout(&self) -> Duration {
        match self {
            StartupPhase::Database { timeout, .. } => *timeout,
            StartupPhase::ImapManager { timeout, .. } => *timeout,
            StartupPhase::AccountSetup { timeout, .. } => *timeout,
            StartupPhase::Services { timeout, .. } => *timeout,
        }
    }

    pub fn is_critical(&self) -> bool {
        match self {
            StartupPhase::Database { .. } => true,
            StartupPhase::ImapManager { .. } => false,
            StartupPhase::AccountSetup { .. } => false,
            StartupPhase::Services { .. } => false,
        }
    }

    pub fn status_icon(&self) -> &'static str {
        match self.status() {
            PhaseStatus::Pending => "⏳",
            PhaseStatus::InProgress { .. } => "🔄",
            PhaseStatus::Completed { .. } => "✅",
            PhaseStatus::Failed { .. } => "❌",
            PhaseStatus::TimedOut { .. } => "⏰",
        }
    }
}

impl PhaseStatus {
    pub fn is_in_progress(&self) -> bool {
        matches!(self, PhaseStatus::InProgress { .. })
    }

    pub fn is_completed(&self) -> bool {
        matches!(self, PhaseStatus::Completed { .. })
    }

    pub fn is_failed(&self) -> bool {
        matches!(self, PhaseStatus::Failed { .. } | PhaseStatus::TimedOut { .. })
    }

    pub fn duration(&self) -> Option<Duration> {
        match self {
            PhaseStatus::Completed { duration } => Some(*duration),
            PhaseStatus::TimedOut { duration } => Some(*duration),
            PhaseStatus::InProgress { started_at } => Some(started_at.elapsed()),
            _ => None,
        }
    }

    pub fn error_message(&self) -> Option<&str> {
        match self {
            PhaseStatus::Failed { error } => Some(error),
            PhaseStatus::TimedOut { .. } => Some("Operation timed out"),
            _ => None,
        }
    }
}

impl Default for StartupPhase {
    fn default() -> Self {
        StartupPhase::Database {
            timeout: Duration::from_secs(30),
            status: PhaseStatus::Pending,
        }
    }
}

impl StartupError {
    pub fn new(phase: String, error: String, is_critical: bool) -> Self {
        Self {
            phase,
            error,
            is_critical,
        }
    }

    pub fn critical(phase: String, error: String) -> Self {
        Self::new(phase, error, true)
    }

    pub fn non_critical(phase: String, error: String) -> Self {
        Self::new(phase, error, false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn test_startup_phase_creation() {
        let phase = StartupPhase::Database {
            timeout: Duration::from_secs(30),
            status: PhaseStatus::Pending,
        };

        assert_eq!(phase.name(), "Database");
        assert_eq!(phase.description(), "Initializing database connection and schema");
        assert_eq!(phase.timeout(), Duration::from_secs(30));
        assert!(phase.is_critical());
        assert_eq!(phase.status_icon(), "⏳");
    }

    #[test]
    fn test_phase_status_transitions() {
        let mut phase = StartupPhase::Database {
            timeout: Duration::from_secs(30),
            status: PhaseStatus::Pending,
        };

        // Test pending state
        assert!(!phase.status().is_in_progress());
        assert!(!phase.status().is_completed());
        assert!(!phase.status().is_failed());

        // Transition to in progress
        *phase.status_mut() = PhaseStatus::InProgress {
            started_at: Instant::now(),
        };

        assert!(phase.status().is_in_progress());
        assert!(!phase.status().is_completed());
        assert!(!phase.status().is_failed());
        assert_eq!(phase.status_icon(), "🔄");

        // Small sleep to get measurable duration
        sleep(Duration::from_millis(10));

        // Transition to completed
        let duration = match phase.status() {
            PhaseStatus::InProgress { started_at } => started_at.elapsed(),
            _ => Duration::from_secs(0),
        };
        *phase.status_mut() = PhaseStatus::Completed { duration };

        assert!(!phase.status().is_in_progress());
        assert!(phase.status().is_completed());
        assert!(!phase.status().is_failed());
        assert_eq!(phase.status_icon(), "✅");
        assert!(phase.status().duration().unwrap() >= Duration::from_millis(10));
    }

    #[test]
    fn test_phase_status_failed() {
        let mut phase = StartupPhase::ImapManager {
            timeout: Duration::from_secs(10),
            status: PhaseStatus::Pending,
        };

        *phase.status_mut() = PhaseStatus::Failed {
            error: "Connection refused".to_string(),
        };

        assert!(phase.status().is_failed());
        assert_eq!(phase.status().error_message(), Some("Connection refused"));
        assert_eq!(phase.status_icon(), "❌");
        assert!(!phase.is_critical());
    }

    #[test]
    fn test_phase_status_timeout() {
        let mut phase = StartupPhase::Services {
            timeout: Duration::from_secs(5),
            status: PhaseStatus::Pending,
        };

        *phase.status_mut() = PhaseStatus::TimedOut {
            duration: Duration::from_secs(5),
        };

        assert!(phase.status().is_failed());
        assert_eq!(phase.status().error_message(), Some("Operation timed out"));
        assert_eq!(phase.status_icon(), "⏰");
        assert_eq!(phase.status().duration(), Some(Duration::from_secs(5)));
    }

    #[test]
    fn test_startup_error_creation() {
        let error = StartupError::critical("Database".to_string(), "Connection failed".to_string());
        assert!(error.is_critical);
        assert_eq!(error.phase, "Database");
        assert_eq!(error.error, "Connection failed");

        let error = StartupError::non_critical("Services".to_string(), "Timeout".to_string());
        assert!(!error.is_critical);
        assert_eq!(error.phase, "Services");
        assert_eq!(error.error, "Timeout");
    }

    #[test]
    fn test_all_phase_types() {
        let phases = vec![
            StartupPhase::Database {
                timeout: Duration::from_secs(30),
                status: PhaseStatus::Pending,
            },
            StartupPhase::ImapManager {
                timeout: Duration::from_secs(10),
                status: PhaseStatus::Pending,
            },
            StartupPhase::AccountSetup {
                timeout: Duration::from_secs(15),
                status: PhaseStatus::Pending,
            },
            StartupPhase::Services {
                timeout: Duration::from_secs(5),
                status: PhaseStatus::Pending,
            },
        ];

        assert_eq!(phases.len(), 4);
        assert_eq!(phases[0].name(), "Database");
        assert_eq!(phases[1].name(), "IMAP Manager");
        assert_eq!(phases[2].name(), "Account Setup");
        assert_eq!(phases[3].name(), "Services");

        // Only database should be critical
        assert!(phases[0].is_critical());
        assert!(!phases[1].is_critical());
        assert!(!phases[2].is_critical());
        assert!(!phases[3].is_critical());
    }
}