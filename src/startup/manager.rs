use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::startup::progress::{PhaseStatus, StartupError, StartupPhase};

pub struct StartupProgressManager {
    phases: Vec<StartupPhase>,
    current_phase: usize,
    started_at: Instant,
    is_visible: bool,
    error_states: HashMap<String, StartupError>,
    phase_logs: HashMap<String, Vec<String>>,
    current_phase_progress: f64,
}

impl StartupProgressManager {
    pub fn new() -> Self {
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
            StartupPhase::DashboardServices {
                timeout: Duration::from_secs(3),
                status: PhaseStatus::Pending,
            },
        ];

        Self {
            phases,
            current_phase: 0,
            started_at: Instant::now(),
            is_visible: true,
            error_states: HashMap::new(),
            phase_logs: HashMap::new(),
            current_phase_progress: 0.0,
        }
    }


    pub fn start_phase(&mut self, phase_name: &str) -> Result<(), String> {
        let phase_index = self.find_phase_by_name(phase_name)?;
        
        // Mark current phase as in progress
        if let Some(phase) = self.phases.get_mut(phase_index) {
            *phase.status_mut() = PhaseStatus::InProgress {
                started_at: Instant::now(),
            };
            self.current_phase = phase_index;
            self.current_phase_progress = 0.0;
            
            // Initialize logs for this phase
            self.phase_logs.insert(phase_name.to_string(), vec![
                format!("🔄 Starting {}", phase_name)
            ]);
            
            Ok(())
        } else {
            Err(format!("Phase {} not found", phase_name))
        }
    }

    pub fn complete_phase(&mut self, phase_name: &str) -> Result<(), String> {
        let phase_index = self.find_phase_by_name(phase_name)?;
        
        if let Some(phase) = self.phases.get_mut(phase_index) {
            let duration = match phase.status() {
                PhaseStatus::InProgress { started_at } => started_at.elapsed(),
                _ => Duration::from_secs(0),
            };
            
            *phase.status_mut() = PhaseStatus::Completed { duration };
            
            // Advance to next phase if this was the current phase
            if phase_index == self.current_phase && self.current_phase + 1 < self.phases.len() {
                self.current_phase += 1;
            }
            
            Ok(())
        } else {
            Err(format!("Phase {} not found", phase_name))
        }
    }

    pub fn fail_phase(&mut self, phase_name: &str, error_message: String) -> Result<(), String> {
        let phase_index = self.find_phase_by_name(phase_name)?;
        
        if let Some(phase) = self.phases.get_mut(phase_index) {
            let is_critical = phase.is_critical();
            
            *phase.status_mut() = PhaseStatus::Failed {
                error: error_message.clone(),
            };

            // Store error state
            let error = if is_critical {
                StartupError::critical(phase_name.to_string(), error_message)
            } else {
                StartupError::non_critical(phase_name.to_string(), error_message)
            };
            self.error_states.insert(phase_name.to_string(), error);

            // Advance to next phase if this was the current phase and it's not critical
            if phase_index == self.current_phase && !is_critical && self.current_phase + 1 < self.phases.len() {
                self.current_phase += 1;
            }
            
            Ok(())
        } else {
            Err(format!("Phase {} not found", phase_name))
        }
    }

    pub fn timeout_phase(&mut self, phase_name: &str) -> Result<(), String> {
        let phase_index = self.find_phase_by_name(phase_name)?;
        
        if let Some(phase) = self.phases.get_mut(phase_index) {
            let duration = match phase.status() {
                PhaseStatus::InProgress { started_at } => started_at.elapsed(),
                _ => phase.timeout(),
            };
            
            let is_critical = phase.is_critical();
            
            *phase.status_mut() = PhaseStatus::TimedOut { duration };

            // Store error state
            let error = if is_critical {
                StartupError::critical(phase_name.to_string(), "Operation timed out".to_string())
            } else {
                StartupError::non_critical(phase_name.to_string(), "Operation timed out".to_string())
            };
            self.error_states.insert(phase_name.to_string(), error);

            // Advance to next phase if this was the current phase and it's not critical
            if phase_index == self.current_phase && !is_critical && self.current_phase + 1 < self.phases.len() {
                self.current_phase += 1;
            }
            
            Ok(())
        } else {
            Err(format!("Phase {} not found", phase_name))
        }
    }

    pub fn overall_progress_percentage(&self) -> f64 {
        if self.phases.is_empty() {
            return 100.0;
        }

        let completed_count = self.phases.iter()
            .filter(|phase| phase.status().is_completed())
            .count();

        let in_progress_weight = if self.current_phase < self.phases.len() {
            match self.phases[self.current_phase].status() {
                PhaseStatus::InProgress { .. } => {
                    // Use current phase progress if available, otherwise fall back to time-based
                    if self.current_phase_progress > 0.0 {
                        (self.current_phase_progress / 100.0).min(0.95) // Max 95% for in-progress
                    } else {
                        let started_at = match self.phases[self.current_phase].status() {
                            PhaseStatus::InProgress { started_at } => *started_at,
                            _ => Instant::now(),
                        };
                        let elapsed = started_at.elapsed();
                        let timeout = self.phases[self.current_phase].timeout();
                        (elapsed.as_secs_f64() / timeout.as_secs_f64()).min(0.8) // Max 80% for time-based
                    }
                }
                _ => 0.0,
            }
        } else {
            0.0
        };

        let total_progress = completed_count as f64 + in_progress_weight;
        (total_progress / self.phases.len() as f64 * 100.0).min(100.0)
    }

    pub fn estimated_time_remaining(&self) -> Option<Duration> {
        if self.is_complete() {
            return Some(Duration::from_secs(0));
        }

        let elapsed = self.started_at.elapsed();
        let progress = self.overall_progress_percentage() / 100.0;
        
        if progress <= 0.0 {
            return None;
        }

        let estimated_total = elapsed.as_secs_f64() / progress;
        let remaining = estimated_total - elapsed.as_secs_f64();
        
        if remaining > 0.0 {
            Some(Duration::from_secs_f64(remaining))
        } else {
            Some(Duration::from_secs(0))
        }
    }

    pub fn total_duration(&self) -> Duration {
        self.started_at.elapsed()
    }

    pub fn current_phase(&self) -> Option<&StartupPhase> {
        self.phases.get(self.current_phase)
    }

    pub fn current_phase_mut(&mut self) -> Option<&mut StartupPhase> {
        self.phases.get_mut(self.current_phase)
    }

    pub fn phases(&self) -> &[StartupPhase] {
        &self.phases
    }

    pub fn is_complete(&self) -> bool {
        self.phases.iter().all(|phase| {
            phase.status().is_completed() || (!phase.is_critical() && phase.status().is_failed())
        })
    }

    pub fn has_critical_errors(&self) -> bool {
        self.error_states.values().any(|error| error.is_critical)
    }

    pub fn error_states(&self) -> &HashMap<String, StartupError> {
        &self.error_states
    }

    pub fn is_visible(&self) -> bool {
        self.is_visible && !self.is_complete()
    }

    pub fn set_visible(&mut self, visible: bool) {
        self.is_visible = visible;
    }

    pub fn hide(&mut self) {
        self.is_visible = false;
    }

    /// Force completion of startup (for when initialization is done but phases weren't properly tracked)
    pub fn force_complete(&mut self) {
        let total_duration = self.started_at.elapsed();
        let phase_duration = Duration::from_millis(total_duration.as_millis() as u64 / self.phases.len() as u64);
        
        // Mark all phases as completed
        for phase in &mut self.phases {
            match phase {
                StartupPhase::Database { status, .. } => *status = PhaseStatus::Completed { duration: phase_duration },
                StartupPhase::ImapManager { status, .. } => *status = PhaseStatus::Completed { duration: phase_duration },
                StartupPhase::AccountSetup { status, .. } => *status = PhaseStatus::Completed { duration: phase_duration },
                StartupPhase::Services { status, .. } => *status = PhaseStatus::Completed { duration: phase_duration },
                StartupPhase::DashboardServices { status, .. } => *status = PhaseStatus::Completed { duration: phase_duration },
            }
        }
        self.current_phase = self.phases.len();
        self.is_visible = false;
    }

    pub fn check_timeout(&mut self, phase_name: &str) -> Result<bool, String> {
        let phase_index = self.find_phase_by_name(phase_name)?;
        
        if let Some(phase) = self.phases.get(phase_index) {
            match phase.status() {
                PhaseStatus::InProgress { started_at } => {
                    let elapsed = started_at.elapsed();
                    Ok(elapsed >= phase.timeout())
                }
                _ => Ok(false),
            }
        } else {
            Err(format!("Phase {} not found", phase_name))
        }
    }

    pub fn time_until_timeout(&self, phase_name: &str) -> Result<Option<Duration>, String> {
        let phase_index = self.find_phase_by_name(phase_name)?;
        
        if let Some(phase) = self.phases.get(phase_index) {
            match phase.status() {
                PhaseStatus::InProgress { started_at } => {
                    let elapsed = started_at.elapsed();
                    let timeout = phase.timeout();
                    if elapsed >= timeout {
                        Ok(Some(Duration::from_secs(0)))
                    } else {
                        Ok(Some(timeout - elapsed))
                    }
                }
                _ => Ok(None),
            }
        } else {
            Err(format!("Phase {} not found", phase_name))
        }
    }

    /// Update progress for the current phase
    pub fn update_phase_progress(&mut self, phase_name: &str, progress: f64, log_message: Option<String>) -> Result<(), String> {
        let phase_index = self.find_phase_by_name(phase_name)?;
        
        if phase_index == self.current_phase {
            self.current_phase_progress = progress.clamp(0.0, 100.0);
            
            // Add log message if provided
            if let Some(message) = log_message {
                self.phase_logs.entry(phase_name.to_string())
                    .or_insert_with(Vec::new)
                    .push(message);
            }
            
            Ok(())
        } else {
            Err(format!("Cannot update progress for non-current phase {}", phase_name))
        }
    }
    
    /// Add a log message to a specific phase
    pub fn add_phase_log(&mut self, phase_name: &str, message: String) {
        self.phase_logs.entry(phase_name.to_string())
            .or_insert_with(Vec::new)
            .push(message);
    }
    
    /// Get logs for a specific phase
    pub fn get_phase_logs(&self, phase_name: &str) -> Vec<String> {
        self.phase_logs.get(phase_name).cloned().unwrap_or_default()
    }
    
    /// Get current phase progress (0-100)
    pub fn current_phase_progress(&self) -> f64 {
        self.current_phase_progress
    }
    
    /// Get current phase name
    pub fn current_phase_name(&self) -> Option<String> {
        self.current_phase().map(|phase| phase.name().to_string())
    }

    fn find_phase_by_name(&self, phase_name: &str) -> Result<usize, String> {
        self.phases
            .iter()
            .position(|phase| phase.name() == phase_name)
            .ok_or_else(|| format!("Phase '{}' not found", phase_name))
    }
}

impl Default for StartupProgressManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn test_startup_progress_manager_creation() {
        let manager = StartupProgressManager::new();
        
        assert_eq!(manager.phases.len(), 5);
        assert_eq!(manager.current_phase, 0);
        assert!(manager.is_visible());
        assert!(!manager.is_complete());
        assert!(!manager.has_critical_errors());
        assert_eq!(manager.overall_progress_percentage(), 0.0);
    }

    #[test]
    fn test_phase_progression() {
        let mut manager = StartupProgressManager::new();
        
        // Start first phase
        manager.start_phase("Database").unwrap();
        assert_eq!(manager.current_phase, 0);
        assert!(manager.current_phase().unwrap().status().is_in_progress());
        
        // Complete first phase
        manager.complete_phase("Database").unwrap();
        assert_eq!(manager.current_phase, 1);
        assert!(manager.phases[0].status().is_completed());
        
        // Start second phase
        manager.start_phase("IMAP Manager").unwrap();
        assert_eq!(manager.current_phase, 1);
        assert!(manager.current_phase().unwrap().status().is_in_progress());
    }

    #[test]
    fn test_progress_calculation() {
        let mut manager = StartupProgressManager::new();
        
        // No progress initially
        assert_eq!(manager.overall_progress_percentage(), 0.0);
        
        // Complete first phase (20% of 5 phases)
        manager.start_phase("Database").unwrap();
        manager.complete_phase("Database").unwrap();
        
        assert_eq!(manager.overall_progress_percentage(), 20.0);
        
        // Complete second phase (40% total)
        manager.start_phase("IMAP Manager").unwrap();
        manager.complete_phase("IMAP Manager").unwrap();
        
        assert_eq!(manager.overall_progress_percentage(), 40.0);
    }

    #[test]
    fn test_error_handling() {
        let mut manager = StartupProgressManager::new();
        
        // Fail a non-critical phase
        manager.start_phase("IMAP Manager").unwrap();
        manager.fail_phase("IMAP Manager", "Connection failed".to_string()).unwrap();
        
        assert!(manager.phases[1].status().is_failed());
        assert!(!manager.has_critical_errors());
        assert_eq!(manager.current_phase, 2); // Should advance to next phase
        
        let errors = manager.error_states();
        assert!(errors.contains_key("IMAP Manager"));
        assert!(!errors["IMAP Manager"].is_critical);
    }

    #[test]
    fn test_critical_error_handling() {
        let mut manager = StartupProgressManager::new();
        
        // Fail the critical database phase
        manager.start_phase("Database").unwrap();
        manager.fail_phase("Database", "Database connection failed".to_string()).unwrap();
        
        assert!(manager.phases[0].status().is_failed());
        assert!(manager.has_critical_errors());
        assert_eq!(manager.current_phase, 0); // Should not advance for critical error
        
        let errors = manager.error_states();
        assert!(errors.contains_key("Database"));
        assert!(errors["Database"].is_critical);
    }

    #[test]
    fn test_timeout_detection() {
        let mut manager = StartupProgressManager::new();
        
        // Start a phase
        manager.start_phase("Services").unwrap();
        
        // Should not be timed out initially
        assert!(!manager.check_timeout("Services").unwrap());
        assert!(manager.time_until_timeout("Services").unwrap().is_some());
        
        // Simulate timeout
        manager.timeout_phase("Services").unwrap();
        assert!(manager.phases[3].status().is_failed());
        assert_eq!(manager.phases[3].status().error_message(), Some("Operation timed out"));
    }

    #[test]
    fn test_completion_detection() {
        let mut manager = StartupProgressManager::new();
        
        assert!(!manager.is_complete());
        
        // Complete all phases
        for phase_name in &["Database", "IMAP Manager", "Account Setup", "Services", "Dashboard Services"] {
            manager.start_phase(phase_name).unwrap();
            manager.complete_phase(phase_name).unwrap();
        }
        
        assert!(manager.is_complete());
        assert_eq!(manager.overall_progress_percentage(), 100.0);
        assert!(!manager.is_visible()); // Should hide when complete
    }

    #[test]
    fn test_partial_completion_with_non_critical_failures() {
        let mut manager = StartupProgressManager::new();
        
        // Complete database (critical)
        manager.start_phase("Database").unwrap();
        manager.complete_phase("Database").unwrap();
        
        // Fail IMAP Manager (non-critical)
        manager.start_phase("IMAP Manager").unwrap();
        manager.fail_phase("IMAP Manager", "Network error".to_string()).unwrap();
        
        // Complete remaining phases
        manager.start_phase("Account Setup").unwrap();
        manager.complete_phase("Account Setup").unwrap();
        
        manager.start_phase("Services").unwrap();
        manager.complete_phase("Services").unwrap();
        
        manager.start_phase("Dashboard Services").unwrap();
        manager.complete_phase("Dashboard Services").unwrap();
        
        // Should be considered complete despite non-critical failure
        assert!(manager.is_complete());
        assert!(!manager.has_critical_errors());
    }

    #[test]
    fn test_estimated_time_remaining() {
        let mut manager = StartupProgressManager::new();
        
        // No progress, no estimate
        assert!(manager.estimated_time_remaining().is_none());
        
        // Complete one phase quickly, then start another
        manager.start_phase("Database").unwrap();
        sleep(Duration::from_millis(10));
        manager.complete_phase("Database").unwrap();
        
        manager.start_phase("IMAP Manager").unwrap();
        sleep(Duration::from_millis(10));
        
        let eta = manager.estimated_time_remaining();
        assert!(eta.is_some());
        // Should have some reasonable estimate based on progress
        assert!(eta.unwrap() > Duration::from_secs(0));
    }

    #[test]
    fn test_visibility_management() {
        let mut manager = StartupProgressManager::new();
        
        assert!(manager.is_visible());
        
        manager.set_visible(false);
        assert!(!manager.is_visible());
        
        manager.set_visible(true);
        assert!(manager.is_visible());
        
        manager.hide();
        assert!(!manager.is_visible());
    }

    #[test]
    fn test_invalid_phase_name() {
        let mut manager = StartupProgressManager::new();
        
        assert!(manager.start_phase("Invalid Phase").is_err());
        assert!(manager.complete_phase("Invalid Phase").is_err());
        assert!(manager.fail_phase("Invalid Phase", "Error".to_string()).is_err());
        assert!(manager.timeout_phase("Invalid Phase").is_err());
        assert!(manager.check_timeout("Invalid Phase").is_err());
        assert!(manager.time_until_timeout("Invalid Phase").is_err());
    }
}