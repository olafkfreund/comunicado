//! Automated backup scheduling system

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ScheduleFrequency {
    Once(DateTime<Utc>),
    Hourly,
    Daily(u8), // hour of day (0-23)
    Weekly(u8, u8), // day of week (0-6), hour (0-23)  
    Monthly(u8, u8), // day of month (1-31), hour (0-23)
    Custom(Duration),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleConfig {
    pub id: Uuid,
    pub name: String,
    pub backup_config_id: Uuid,
    pub frequency: ScheduleFrequency,
    pub enabled: bool,
    pub next_run: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledTask {
    pub id: Uuid,
    pub schedule_id: Uuid,
    pub scheduled_time: DateTime<Utc>,
    pub status: TaskStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed(String),
}

pub struct BackupScheduler {
    schedules: Vec<ScheduleConfig>,
    pending_tasks: Vec<ScheduledTask>,
}

impl BackupScheduler {
    pub fn new() -> Self {
        Self {
            schedules: Vec::new(),
            pending_tasks: Vec::new(),
        }
    }

    pub fn add_schedule(&mut self, schedule: ScheduleConfig) {
        self.schedules.push(schedule);
    }

    pub fn get_due_tasks(&self) -> Vec<&ScheduledTask> {
        let now = Utc::now();
        self.pending_tasks
            .iter()
            .filter(|task| task.scheduled_time <= now && task.status == TaskStatus::Pending)
            .collect()
    }
}

impl Default for BackupScheduler {
    fn default() -> Self {
        Self::new()
    }
}