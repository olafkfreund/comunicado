use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::ui::start_page::{TaskItem, TaskPriority};

/// Task management service
pub struct TaskService {
    tasks: Vec<TaskItem>,
    data_file: PathBuf,
    auto_save: bool,
}

/// Serializable task for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SerializableTask {
    id: String,
    title: String,
    completed: bool,
    priority: String,
    due_date: Option<String>,
    created_at: String,
    completed_at: Option<String>,
}

impl From<&TaskItem> for SerializableTask {
    fn from(task: &TaskItem) -> Self {
        Self {
            id: task.id.clone(),
            title: task.title.clone(),
            completed: task.completed,
            priority: match task.priority {
                TaskPriority::Low => "low".to_string(),
                TaskPriority::Medium => "medium".to_string(),
                TaskPriority::High => "high".to_string(),
                TaskPriority::Urgent => "urgent".to_string(),
            },
            due_date: task.due_date.map(|d| d.to_rfc3339()),
            created_at: Local::now().to_rfc3339(),
            completed_at: if task.completed {
                Some(Local::now().to_rfc3339())
            } else {
                None
            },
        }
    }
}

impl TryFrom<SerializableTask> for TaskItem {
    type Error = Box<dyn std::error::Error>;

    fn try_from(task: SerializableTask) -> Result<Self, Self::Error> {
        let priority = match task.priority.as_str() {
            "low" => TaskPriority::Low,
            "medium" => TaskPriority::Medium,
            "high" => TaskPriority::High,
            "urgent" => TaskPriority::Urgent,
            _ => TaskPriority::Medium,
        };

        let due_date = if let Some(due_str) = task.due_date {
            Some(DateTime::parse_from_rfc3339(&due_str)?.with_timezone(&Local))
        } else {
            None
        };

        Ok(TaskItem {
            id: task.id,
            title: task.title,
            completed: task.completed,
            priority,
            due_date,
        })
    }
}

impl TaskService {
    /// Create a new task service
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let data_dir = dirs::data_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("comunicado");
        
        // Create directory if it doesn't exist
        std::fs::create_dir_all(&data_dir)?;
        
        let data_file = data_dir.join("tasks.json");

        let mut service = Self {
            tasks: Vec::new(),
            data_file,
            auto_save: true,
        };

        // Load existing tasks
        service.load_tasks()?;

        // Add some default tasks if none exist
        if service.tasks.is_empty() {
            service.add_default_tasks()?;
        }

        Ok(service)
    }

    /// Add default tasks for demonstration
    fn add_default_tasks(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let default_tasks = vec![
            TaskItem {
                id: Uuid::new_v4().to_string(),
                title: "Check emails".to_string(),
                completed: false,
                priority: TaskPriority::Medium,
                due_date: Some(Local::now() + chrono::Duration::hours(2)),
            },
            TaskItem {
                id: Uuid::new_v4().to_string(),
                title: "Review project roadmap".to_string(),
                completed: false,
                priority: TaskPriority::High,
                due_date: Some(Local::now() + chrono::Duration::days(1)),
            },
            TaskItem {
                id: Uuid::new_v4().to_string(),
                title: "Schedule team meeting".to_string(),
                completed: true,
                priority: TaskPriority::Low,
                due_date: None,
            },
            TaskItem {
                id: Uuid::new_v4().to_string(),
                title: "Update documentation".to_string(),
                completed: false,
                priority: TaskPriority::Medium,
                due_date: Some(Local::now() + chrono::Duration::days(3)),
            },
            TaskItem {
                id: Uuid::new_v4().to_string(),
                title: "Backup important files".to_string(),
                completed: false,
                priority: TaskPriority::Urgent,
                due_date: Some(Local::now() + chrono::Duration::hours(6)),
            },
        ];

        for task in default_tasks {
            self.tasks.push(task);
        }

        if self.auto_save {
            self.save_tasks()?;
        }

        Ok(())
    }

    /// Get all tasks
    pub fn get_tasks(&self) -> &Vec<TaskItem> {
        &self.tasks
    }

    /// Get incomplete tasks only
    pub fn get_pending_tasks(&self) -> Vec<&TaskItem> {
        self.tasks.iter().filter(|task| !task.completed).collect()
    }

    /// Get completed tasks only
    pub fn get_completed_tasks(&self) -> Vec<&TaskItem> {
        self.tasks.iter().filter(|task| task.completed).collect()
    }

    /// Get tasks by priority
    pub fn get_tasks_by_priority(&self, priority: TaskPriority) -> Vec<&TaskItem> {
        self.tasks.iter().filter(|task| task.priority == priority).collect()
    }

    /// Get tasks due today
    pub fn get_tasks_due_today(&self) -> Vec<&TaskItem> {
        let today = Local::now().date_naive();
        self.tasks.iter()
            .filter(|task| {
                if let Some(due_date) = task.due_date {
                    due_date.date_naive() == today
                } else {
                    false
                }
            })
            .collect()
    }

    /// Get overdue tasks
    pub fn get_overdue_tasks(&self) -> Vec<&TaskItem> {
        let now = Local::now();
        self.tasks.iter()
            .filter(|task| {
                !task.completed && task.due_date.map_or(false, |due| due < now)
            })
            .collect()
    }

    /// Add a new task
    pub fn add_task(&mut self, mut task: TaskItem) -> Result<(), Box<dyn std::error::Error>> {
        // Generate ID if not provided
        if task.id.is_empty() {
            task.id = Uuid::new_v4().to_string();
        }

        self.tasks.push(task);

        if self.auto_save {
            self.save_tasks()?;
        }

        Ok(())
    }

    /// Create and add a simple task
    pub fn add_simple_task(
        &mut self, 
        title: String, 
        priority: TaskPriority,
        due_date: Option<DateTime<Local>>
    ) -> Result<String, Box<dyn std::error::Error>> {
        let task_id = Uuid::new_v4().to_string();
        let task = TaskItem {
            id: task_id.clone(),
            title,
            completed: false,
            priority,
            due_date,
        };

        self.add_task(task)?;
        Ok(task_id)
    }

    /// Toggle task completion
    pub fn toggle_task(&mut self, task_id: &str) -> Result<bool, Box<dyn std::error::Error>> {
        if let Some(task) = self.tasks.iter_mut().find(|t| t.id == task_id) {
            task.completed = !task.completed;
            let completed = task.completed;
            
            if self.auto_save {
                self.save_tasks()?;
            }
            
            Ok(completed)
        } else {
            Err(format!("Task with id {} not found", task_id).into())
        }
    }

    /// Update task priority
    pub fn update_task_priority(&mut self, task_id: &str, priority: TaskPriority) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(task) = self.tasks.iter_mut().find(|t| t.id == task_id) {
            task.priority = priority;
            
            if self.auto_save {
                self.save_tasks()?;
            }
            
            Ok(())
        } else {
            Err(format!("Task with id {} not found", task_id).into())
        }
    }

    /// Update task due date
    pub fn update_task_due_date(&mut self, task_id: &str, due_date: Option<DateTime<Local>>) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(task) = self.tasks.iter_mut().find(|t| t.id == task_id) {
            task.due_date = due_date;
            
            if self.auto_save {
                self.save_tasks()?;
            }
            
            Ok(())
        } else {
            Err(format!("Task with id {} not found", task_id).into())
        }
    }

    /// Delete a task
    pub fn delete_task(&mut self, task_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let initial_len = self.tasks.len();
        self.tasks.retain(|task| task.id != task_id);
        
        if self.tasks.len() < initial_len {
            if self.auto_save {
                self.save_tasks()?;
            }
            Ok(())
        } else {
            Err(format!("Task with id {} not found", task_id).into())
        }
    }

    /// Clear all completed tasks
    pub fn clear_completed(&mut self) -> Result<usize, Box<dyn std::error::Error>> {
        let initial_len = self.tasks.len();
        self.tasks.retain(|task| !task.completed);
        let removed_count = initial_len - self.tasks.len();
        
        if removed_count > 0 && self.auto_save {
            self.save_tasks()?;
        }
        
        Ok(removed_count)
    }

    /// Get task statistics
    pub fn get_stats(&self) -> HashMap<String, usize> {
        let mut stats = HashMap::new();
        
        stats.insert("total".to_string(), self.tasks.len());
        stats.insert("completed".to_string(), self.get_completed_tasks().len());
        stats.insert("pending".to_string(), self.get_pending_tasks().len());
        stats.insert("overdue".to_string(), self.get_overdue_tasks().len());
        stats.insert("due_today".to_string(), self.get_tasks_due_today().len());
        
        stats.insert("low_priority".to_string(), self.get_tasks_by_priority(TaskPriority::Low).len());
        stats.insert("medium_priority".to_string(), self.get_tasks_by_priority(TaskPriority::Medium).len());
        stats.insert("high_priority".to_string(), self.get_tasks_by_priority(TaskPriority::High).len());
        stats.insert("urgent_priority".to_string(), self.get_tasks_by_priority(TaskPriority::Urgent).len());
        
        stats
    }

    /// Save tasks to file
    pub fn save_tasks(&self) -> Result<(), Box<dyn std::error::Error>> {
        let serializable_tasks: Vec<SerializableTask> = self.tasks.iter()
            .map(SerializableTask::from)
            .collect();

        let json = serde_json::to_string_pretty(&serializable_tasks)?;
        fs::write(&self.data_file, json)?;
        
        tracing::debug!("Saved {} tasks to {:?}", self.tasks.len(), self.data_file);
        Ok(())
    }

    /// Load tasks from file
    pub fn load_tasks(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if !self.data_file.exists() {
            tracing::debug!("Task file {:?} does not exist, starting with empty tasks", self.data_file);
            return Ok(());
        }

        let json = fs::read_to_string(&self.data_file)?;
        let serializable_tasks: Vec<SerializableTask> = serde_json::from_str(&json)?;
        
        self.tasks.clear();
        for serializable_task in serializable_tasks {
            match TaskItem::try_from(serializable_task) {
                Ok(task) => self.tasks.push(task),
                Err(e) => tracing::warn!("Failed to deserialize task: {}", e),
            }
        }
        
        tracing::debug!("Loaded {} tasks from {:?}", self.tasks.len(), self.data_file);
        Ok(())
    }

    /// Set auto-save mode
    pub fn set_auto_save(&mut self, auto_save: bool) {
        self.auto_save = auto_save;
    }

    /// Manual save (useful when auto_save is disabled)
    pub fn manual_save(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.save_tasks()
    }
}

impl Default for TaskService {
    fn default() -> Self {
        Self::new().unwrap_or_else(|e| {
            tracing::error!("Failed to initialize task service: {}", e);
            Self {
                tasks: Vec::new(),
                data_file: PathBuf::from("tasks.json"),
                auto_save: true,
            }
        })
    }
}