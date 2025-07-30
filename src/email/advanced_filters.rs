//! Advanced email filtering system with complex rules and boolean logic
//!
//! This module extends the basic filtering system with support for:
//! - Complex boolean logic (AND, OR, NOT)
//! - Nested condition groups
//! - Time-based conditions
//! - Conditional actions
//! - Filter templates and presets

use crate::email::StoredMessage;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Advanced email filter with complex boolean logic
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedEmailFilter {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub enabled: bool,
    pub priority: i32,
    pub condition_group: ConditionGroup,
    pub action_rules: Vec<ActionRule>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub author: String,
    pub tags: Vec<String>,
    pub statistics: FilterStatistics,
}

/// Group of conditions with boolean logic
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionGroup {
    pub logic: BooleanLogic,
    pub conditions: Vec<AdvancedCondition>,
    pub nested_groups: Vec<ConditionGroup>,
}

/// Boolean logic operators for combining conditions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BooleanLogic {
    And,
    Or,
    Not,
}

/// Advanced condition with extended field types and operators
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedCondition {
    pub field: AdvancedFilterField,
    pub operator: AdvancedFilterOperator,
    pub value: FilterValue,
    pub case_sensitive: bool,
    pub negate: bool, // Apply NOT to this condition
}

/// Extended filter fields with time-based and metadata fields
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AdvancedFilterField {
    // Basic fields (inherited)
    From,
    To,
    CC,
    BCC,
    Subject,
    Body,
    Sender,
    ReplyTo,
    MessageId,
    InReplyTo,
    HasAttachment,
    AttachmentName,
    AttachmentType,
    AttachmentSize,
    Size,
    Date,
    Priority,
    Folder,
    
    // Advanced fields
    ReceivedDate,
    SentDate,
    Age,                    // Days since received
    TimeOfDay,             // Hour of day (0-23)
    DayOfWeek,             // 0=Sunday, 6=Saturday
    DayOfMonth,            // 1-31
    Month,                 // 1-12
    Year,                  // YYYY
    IsRead,
    IsImportant,
    IsSpam,
    HasFlag(String),       // Custom flag
    ThreadCount,           // Number of messages in thread
    AttachmentCount,
    WordCount,
    HeaderField(String),   // Custom header field
    ListId,                // Mailing list ID
    FromDomain,
    ToDomain,
    Language,              // Detected language
    SpamScore,
    VirusCheckStatus,
    IsEncrypted,
    IsSigned,
    CertificateValid,
}

/// Advanced filter operators with regex and numerical comparisons
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AdvancedFilterOperator {
    // String operators
    Contains,
    NotContains,
    StartsWith,
    EndsWith,
    Equals,
    NotEquals,
    Regex,
    GlobPattern,
    IsEmpty,
    IsNotEmpty,
    
    // Numerical operators
    GreaterThan,
    LessThan,
    GreaterThanOrEqual,
    LessThanOrEqual,
    NumericBetween(f64, f64),     // min, max for numbers
    
    // Set operators
    In(Vec<String>),       // Value is in list
    NotIn(Vec<String>),    // Value is not in list
    
    // Time operators
    Within(TimePeriod),    // Within time period
    Before(DateTime<Utc>),
    After(DateTime<Utc>),
    DateTimeBetween(DateTime<Utc>, DateTime<Utc>),
    
    // Boolean operators
    IsTrue,
    IsFalse,
}

/// Time period specifications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TimePeriod {
    Minutes(i64),
    Hours(i64),
    Days(i64),
    Weeks(i64),
    Months(i64),
    Years(i64),
}

/// Filter value that can be different types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilterValue {
    String(String),
    Number(f64),
    Boolean(bool),
    DateTime(DateTime<Utc>),
    List(Vec<String>),
    Regex(String),
    TimePeriod(TimePeriod),
}

/// Action rule with conditions for when to apply
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionRule {
    pub actions: Vec<AdvancedFilterAction>,
    pub condition: Option<ConditionGroup>, // Optional condition for this action
    pub priority: i32,
    pub enabled: bool,
}

/// Extended filter actions with more options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AdvancedFilterAction {
    // Basic actions (inherited)
    MoveToFolder(String),
    CopyToFolder(String),
    AddLabel(String),
    RemoveLabel(String),
    MarkAsRead,
    MarkAsUnread,
    MarkAsImportant,
    MarkAsSpam,
    Delete,
    Forward(String),
    AutoReply(String),
    SetFlag(String),
    RemoveFlag(String),
    StopProcessing,
    
    // Advanced actions
    ForwardWithTemplate {
        to: String,
        template: String,
        attach_original: bool,
    },
    AutoReplyWithTemplate {
        template: String,
        once_per_sender: bool,
        delay_minutes: Option<u32>,
    },
    RunScript(String),           // Execute external script
    TriggerWebhook {
        url: String,
        method: String,
        headers: HashMap<String, String>,
        body_template: Option<String>,
    },
    PlaySound(String),           // Sound file path
    ShowNotification {
        title: String,
        message: String,
        priority: NotificationPriority,
    },
    AddToCalendar {              // Create calendar event from email
        title_template: String,
        duration_minutes: u32,
    },
    ExtractData {               // Extract data using regex
        field: String,
        regex: String,
        action: DataExtractionAction,
    },
    SetPriority(MessagePriority),
    AddToAddressBook,           // Add sender to contacts
    BlockSender,                // Add sender to blocklist
    ConditionalAction {         // Apply action based on condition
        condition: ConditionGroup,
        action: Box<AdvancedFilterAction>,
        else_action: Option<Box<AdvancedFilterAction>>,
    },
}

/// Notification priority levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationPriority {
    Low,
    Normal,
    High,
    Critical,
}

/// Message priority levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessagePriority {
    Low,
    Normal,
    High,
    Urgent,
}

/// Data extraction actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataExtractionAction {
    SaveToFile(String),         // File path
    SaveToDatabase(String),     // Table/collection name
    SendToWebhook(String),      // Webhook URL
    LogToConsole,
}

/// Filter statistics for performance monitoring
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FilterStatistics {
    pub matches_count: u64,
    pub actions_executed: u64,
    pub last_matched: Option<DateTime<Utc>>,
    pub average_execution_time_ms: f64,
    pub errors_count: u64,
    pub last_error: Option<String>,
}

/// Advanced filter engine with complex rule evaluation
pub struct AdvancedFilterEngine {
    filters: Vec<AdvancedEmailFilter>,
    templates: FilterTemplateLibrary,
}

/// Result of advanced filter processing
#[derive(Debug, Clone)]
pub struct AdvancedFilterResult {
    pub matched_filters: Vec<Uuid>,
    pub actions_applied: Vec<AdvancedFilterAction>,
    pub stop_processing: bool,
    pub execution_time_ms: u64,
    pub errors: Vec<FilterError>,
}

/// Filter execution errors
#[derive(Debug, Clone)]
pub struct FilterError {
    pub filter_id: Uuid,
    pub action: Option<AdvancedFilterAction>,
    pub error_message: String,
    pub timestamp: DateTime<Utc>,
}

impl AdvancedFilterEngine {
    /// Create a new advanced filter engine
    pub fn new() -> Self {
        Self {
            filters: Vec::new(),
            templates: FilterTemplateLibrary::new(),
        }
    }

    /// Load filters from database/storage
    pub fn load_filters(&mut self, filters: Vec<AdvancedEmailFilter>) {
        self.filters = filters;
        // Sort by priority (lower number = higher priority)
        self.filters.sort_by_key(|f| f.priority);
    }

    /// Add a new filter
    pub fn add_filter(&mut self, filter: AdvancedEmailFilter) {
        self.filters.push(filter);
        self.filters.sort_by_key(|f| f.priority);
    }

    /// Process a message through all filters
    pub async fn process_message(&self, message: &StoredMessage) -> AdvancedFilterResult {
        let start_time = std::time::Instant::now();
        let mut result = AdvancedFilterResult {
            matched_filters: Vec::new(),
            actions_applied: Vec::new(),
            stop_processing: false,
            execution_time_ms: 0,
            errors: Vec::new(),
        };

        for filter in &self.filters {
            if !filter.enabled {
                continue;
            }

            match self.evaluate_condition_group(&filter.condition_group, message) {
                Ok(matches) => {
                    if matches {
                        result.matched_filters.push(filter.id);

                        // Apply action rules in priority order
                        let mut action_rules = filter.action_rules.clone();
                        action_rules.sort_by_key(|ar| ar.priority);

                        for action_rule in &action_rules {
                            if !action_rule.enabled {
                                continue;
                            }

                            // Check if action rule has its own condition
                            let apply_actions = if let Some(condition) = &action_rule.condition {
                                match self.evaluate_condition_group(condition, message) {
                                    Ok(matches) => matches,
                                    Err(error) => {
                                        result.errors.push(FilterError {
                                            filter_id: filter.id,
                                            action: None,
                                            error_message: format!("Action condition error: {}", error),
                                            timestamp: Utc::now(),
                                        });
                                        false
                                    }
                                }
                            } else {
                                true
                            };

                            if apply_actions {
                                for action in &action_rule.actions {
                                    if matches!(action, AdvancedFilterAction::StopProcessing) {
                                        result.stop_processing = true;
                                        break;
                                    }
                                    result.actions_applied.push(action.clone());
                                }
                            }

                            if result.stop_processing {
                                break;
                            }
                        }

                        if result.stop_processing {
                            break;
                        }
                    }
                }
                Err(error) => {
                    result.errors.push(FilterError {
                        filter_id: filter.id,
                        action: None,
                        error_message: format!("Filter evaluation error: {}", error),
                        timestamp: Utc::now(),
                    });
                }
            }
        }

        result.execution_time_ms = start_time.elapsed().as_millis() as u64;
        result
    }

    /// Evaluate a condition group with boolean logic
    fn evaluate_condition_group(
        &self,
        group: &ConditionGroup,
        message: &StoredMessage,
    ) -> Result<bool, String> {
        let mut condition_results = Vec::new();

        // Evaluate individual conditions
        for condition in &group.conditions {
            let result = self.evaluate_advanced_condition(condition, message)?;
            condition_results.push(result);
        }

        // Evaluate nested groups
        for nested_group in &group.nested_groups {
            let result = self.evaluate_condition_group(nested_group, message)?;
            condition_results.push(result);
        }

        // Apply boolean logic
        match group.logic {
            BooleanLogic::And => Ok(condition_results.iter().all(|&r| r)),
            BooleanLogic::Or => Ok(condition_results.iter().any(|&r| r)),
            BooleanLogic::Not => {
                // NOT logic applies to the entire group
                let all_true = condition_results.iter().all(|&r| r);
                Ok(!all_true)
            }
        }
    }

    /// Evaluate a single advanced condition
    fn evaluate_advanced_condition(
        &self,
        condition: &AdvancedCondition,
        message: &StoredMessage,
    ) -> Result<bool, String> {
        let field_value = self.extract_advanced_field_value(&condition.field, message)?;
        let result = self.apply_advanced_operator(&condition.operator, &field_value, &condition.value, condition.case_sensitive)?;
        
        // Apply negation if specified
        Ok(if condition.negate { !result } else { result })
    }

    /// Extract value from advanced fields
    fn extract_advanced_field_value(
        &self,
        field: &AdvancedFilterField,
        message: &StoredMessage,
    ) -> Result<FilterValue, String> {
        match field {
            // Basic fields
            AdvancedFilterField::From => Ok(FilterValue::String(message.from_addr.clone())),
            AdvancedFilterField::To => Ok(FilterValue::String(message.to_addrs.join(", "))),
            AdvancedFilterField::Subject => Ok(FilterValue::String(message.subject.clone())),
            AdvancedFilterField::Body => Ok(FilterValue::String(
                message.body_text.as_deref().unwrap_or("").to_string()
            )),
            AdvancedFilterField::Date => Ok(FilterValue::DateTime(message.date)),
            AdvancedFilterField::Size => Ok(FilterValue::Number(
                message.size.map(|s| s as f64).unwrap_or(0.0)
            )),
            AdvancedFilterField::IsRead => Ok(FilterValue::Boolean(
                message.flags.contains(&"\\Seen".to_string())
            )),
            AdvancedFilterField::IsImportant => Ok(FilterValue::Boolean(
                message.flags.contains(&"\\Flagged".to_string())
            )),
            
            // Time-based fields
            AdvancedFilterField::Age => {
                let age_days = (Utc::now() - message.date).num_days();
                Ok(FilterValue::Number(age_days as f64))
            }
            AdvancedFilterField::TimeOfDay => {
                Ok(FilterValue::Number(message.date.hour() as f64))
            }
            AdvancedFilterField::DayOfWeek => {
                Ok(FilterValue::Number(message.date.weekday().num_days_from_sunday() as f64))
            }
            AdvancedFilterField::DayOfMonth => {
                Ok(FilterValue::Number(message.date.day() as f64))
            }
            AdvancedFilterField::Month => {
                Ok(FilterValue::Number(message.date.month() as f64))
            }
            AdvancedFilterField::Year => {
                Ok(FilterValue::Number(message.date.year() as f64))
            }
            
            // Attachment fields
            AdvancedFilterField::HasAttachment => {
                Ok(FilterValue::Boolean(!message.attachments.is_empty()))
            }
            AdvancedFilterField::AttachmentCount => {
                Ok(FilterValue::Number(message.attachments.len() as f64))
            }
            
            // Domain extraction
            AdvancedFilterField::FromDomain => {
                let domain = message.from_addr.split('@').nth(1).unwrap_or("").to_string();
                Ok(FilterValue::String(domain))
            }
            AdvancedFilterField::ToDomain => {
                if let Some(first_to) = message.to_addrs.first() {
                    let domain = first_to.split('@').nth(1).unwrap_or("").to_string();
                    Ok(FilterValue::String(domain))
                } else {
                    Ok(FilterValue::String(String::new()))
                }
            }
            
            // Word count
            AdvancedFilterField::WordCount => {
                let word_count = message.body_text.as_deref().unwrap_or("").split_whitespace().count();
                Ok(FilterValue::Number(word_count as f64))
            }
            
            _ => Err(format!("Field {:?} not yet implemented", field)),
        }
    }

    /// Apply advanced operator to values
    fn apply_advanced_operator(
        &self,
        operator: &AdvancedFilterOperator,
        field_value: &FilterValue,
        condition_value: &FilterValue,
        case_sensitive: bool,
    ) -> Result<bool, String> {
        match (field_value, condition_value, operator) {
            // String operations
            (FilterValue::String(field), FilterValue::String(condition), op) => {
                let field_str = if case_sensitive { field.clone() } else { field.to_lowercase() };
                let condition_str = if case_sensitive { condition.clone() } else { condition.to_lowercase() };
                
                match op {
                    AdvancedFilterOperator::Contains => Ok(field_str.contains(&condition_str)),
                    AdvancedFilterOperator::NotContains => Ok(!field_str.contains(&condition_str)),
                    AdvancedFilterOperator::StartsWith => Ok(field_str.starts_with(&condition_str)),
                    AdvancedFilterOperator::EndsWith => Ok(field_str.ends_with(&condition_str)),
                    AdvancedFilterOperator::Equals => Ok(field_str == condition_str),
                    AdvancedFilterOperator::NotEquals => Ok(field_str != condition_str),
                    AdvancedFilterOperator::IsEmpty => Ok(field_str.is_empty()),
                    AdvancedFilterOperator::IsNotEmpty => Ok(!field_str.is_empty()),
                    AdvancedFilterOperator::Regex => {
                        // TODO: Implement regex matching
                        Ok(false)
                    }
                    _ => Err(format!("Operator {:?} not supported for string values", op)),
                }
            }
            
            // Numerical operations
            (FilterValue::Number(field), FilterValue::Number(condition), op) => {
                match op {
                    AdvancedFilterOperator::GreaterThan => Ok(field > condition),
                    AdvancedFilterOperator::LessThan => Ok(field < condition),
                    AdvancedFilterOperator::GreaterThanOrEqual => Ok(field >= condition),
                    AdvancedFilterOperator::LessThanOrEqual => Ok(field <= condition),
                    AdvancedFilterOperator::Equals => Ok((field - condition).abs() < f64::EPSILON),
                    AdvancedFilterOperator::NotEquals => Ok((field - condition).abs() >= f64::EPSILON),
                    AdvancedFilterOperator::NumericBetween(min, max) => Ok(field >= min && field <= max),
                    _ => Err(format!("Operator {:?} not supported for numeric values", op)),
                }
            }
            
            // Boolean operations
            (FilterValue::Boolean(field), FilterValue::Boolean(condition), op) => {
                match op {
                    AdvancedFilterOperator::Equals => Ok(field == condition),
                    AdvancedFilterOperator::NotEquals => Ok(field != condition),
                    AdvancedFilterOperator::IsTrue => Ok(*field),
                    AdvancedFilterOperator::IsFalse => Ok(!*field),
                    _ => Err(format!("Operator {:?} not supported for boolean values", op)),
                }
            }
            
            // DateTime operations
            (FilterValue::DateTime(field), FilterValue::DateTime(condition), op) => {
                match op {
                    AdvancedFilterOperator::Before(_) => Ok(field < condition),
                    AdvancedFilterOperator::After(_) => Ok(field > condition),
                    AdvancedFilterOperator::Equals => Ok(field == condition),
                    _ => Err(format!("Operator {:?} not supported for datetime values", op)),
                }
            }
            
            _ => Err(format!("Unsupported value type combination for operator {:?}", operator)),
        }
    }

    /// Get filter templates
    pub fn get_templates(&self) -> &FilterTemplateLibrary {
        &self.templates
    }
}

/// Library of pre-defined filter templates
pub struct FilterTemplateLibrary {
    templates: HashMap<String, AdvancedEmailFilter>,
}

impl FilterTemplateLibrary {
    pub fn new() -> Self {
        let mut templates = HashMap::new();
        
        // Add common filter templates
        templates.insert("spam_filter".to_string(), Self::create_spam_filter_template());
        templates.insert("newsletter_organizer".to_string(), Self::create_newsletter_organizer_template());
        templates.insert("important_clients".to_string(), Self::create_important_clients_template());
        templates.insert("weekend_backup".to_string(), Self::create_weekend_backup_template());
        
        Self { templates }
    }

    /// Get all available templates
    pub fn get_templates(&self) -> &HashMap<String, AdvancedEmailFilter> {
        &self.templates
    }

    /// Get a specific template
    pub fn get_template(&self, name: &str) -> Option<&AdvancedEmailFilter> {
        self.templates.get(name)
    }

    /// Create spam filter template
    fn create_spam_filter_template() -> AdvancedEmailFilter {
        AdvancedEmailFilter {
            id: Uuid::new_v4(),
            name: "Spam Filter".to_string(),
            description: "Automatically detect and move spam emails".to_string(),
            enabled: true,
            priority: 1,
            condition_group: ConditionGroup {
                logic: BooleanLogic::Or,
                conditions: vec![
                    AdvancedCondition {
                        field: AdvancedFilterField::Subject,
                        operator: AdvancedFilterOperator::Contains,
                        value: FilterValue::String("URGENT".to_string()),
                        case_sensitive: false,
                        negate: false,
                    },
                    AdvancedCondition {
                        field: AdvancedFilterField::Subject,
                        operator: AdvancedFilterOperator::Contains,
                        value: FilterValue::String("FREE MONEY".to_string()),
                        case_sensitive: false,
                        negate: false,
                    },
                ],
                nested_groups: vec![],
            },
            action_rules: vec![ActionRule {
                actions: vec![AdvancedFilterAction::MarkAsSpam, AdvancedFilterAction::MoveToFolder("Spam".to_string())],
                condition: None,
                priority: 1,
                enabled: true,
            }],
            created_at: Utc::now(),
            updated_at: Utc::now(),
            author: "System".to_string(),
            tags: vec!["spam".to_string(), "security".to_string()],
            statistics: FilterStatistics::default(),
        }
    }

    /// Create newsletter organizer template
    fn create_newsletter_organizer_template() -> AdvancedEmailFilter {
        AdvancedEmailFilter {
            id: Uuid::new_v4(),
            name: "Newsletter Organizer".to_string(),
            description: "Organize newsletters and mailing lists".to_string(),
            enabled: true,
            priority: 5,
            condition_group: ConditionGroup {
                logic: BooleanLogic::Or,
                conditions: vec![
                    AdvancedCondition {
                        field: AdvancedFilterField::Subject,
                        operator: AdvancedFilterOperator::Contains,
                        value: FilterValue::String("newsletter".to_string()),
                        case_sensitive: false,
                        negate: false,
                    },
                    AdvancedCondition {
                        field: AdvancedFilterField::Subject,
                        operator: AdvancedFilterOperator::Contains,
                        value: FilterValue::String("unsubscribe".to_string()),
                        case_sensitive: false,
                        negate: false,
                    },
                ],
                nested_groups: vec![],
            },
            action_rules: vec![ActionRule {
                actions: vec![
                    AdvancedFilterAction::MoveToFolder("Newsletters".to_string()),
                    AdvancedFilterAction::AddLabel("newsletter".to_string()),
                ],
                condition: None,
                priority: 1,
                enabled: true,
            }],
            created_at: Utc::now(),
            updated_at: Utc::now(),
            author: "System".to_string(),
            tags: vec!["newsletter".to_string(), "organization".to_string()],
            statistics: FilterStatistics::default(),
        }
    }

    /// Create important clients template
    fn create_important_clients_template() -> AdvancedEmailFilter {
        AdvancedEmailFilter {
            id: Uuid::new_v4(),
            name: "Important Clients".to_string(),
            description: "Prioritize emails from important clients".to_string(),
            enabled: true,
            priority: 2,
            condition_group: ConditionGroup {
                logic: BooleanLogic::Or,
                conditions: vec![
                    AdvancedCondition {
                        field: AdvancedFilterField::FromDomain,
                        operator: AdvancedFilterOperator::In(vec![
                            "bigcorp.com".to_string(),
                            "majorpartner.com".to_string(),
                            "keyclient.org".to_string(),
                        ]),
                        value: FilterValue::List(vec![]),
                        case_sensitive: false,
                        negate: false,
                    },
                ],
                nested_groups: vec![],
            },
            action_rules: vec![ActionRule {
                actions: vec![
                    AdvancedFilterAction::MarkAsImportant,
                    AdvancedFilterAction::AddLabel("VIP".to_string()),
                    AdvancedFilterAction::ShowNotification {
                        title: "Important Email".to_string(),
                        message: "Email from VIP client received".to_string(),
                        priority: NotificationPriority::High,
                    },
                ],
                condition: None,
                priority: 1,
                enabled: true,
            }],
            created_at: Utc::now(),
            updated_at: Utc::now(),
            author: "System".to_string(),
            tags: vec!["vip".to_string(), "clients".to_string()],
            statistics: FilterStatistics::default(),
        }
    }

    /// Create weekend backup template
    fn create_weekend_backup_template() -> AdvancedEmailFilter {
        AdvancedEmailFilter {
            id: Uuid::new_v4(),
            name: "Weekend Email Backup".to_string(),
            description: "Auto-organize weekend emails for Monday review".to_string(),
            enabled: true,
            priority: 10,
            condition_group: ConditionGroup {
                logic: BooleanLogic::Or,
                conditions: vec![
                    AdvancedCondition {
                        field: AdvancedFilterField::DayOfWeek,
                        operator: AdvancedFilterOperator::Equals,
                        value: FilterValue::Number(0.0), // Sunday
                        case_sensitive: false,
                        negate: false,
                    },
                    AdvancedCondition {
                        field: AdvancedFilterField::DayOfWeek,
                        operator: AdvancedFilterOperator::Equals,
                        value: FilterValue::Number(6.0), // Saturday
                        case_sensitive: false,
                        negate: false,
                    },
                ],
                nested_groups: vec![],
            },
            action_rules: vec![ActionRule {
                actions: vec![
                    AdvancedFilterAction::AddLabel("weekend".to_string()),
                    AdvancedFilterAction::MoveToFolder("Weekend Review".to_string()),
                ],
                condition: None,
                priority: 1,
                enabled: true,
            }],
            created_at: Utc::now(),
            updated_at: Utc::now(),
            author: "System".to_string(),
            tags: vec!["weekend".to_string(), "organization".to_string()],
            statistics: FilterStatistics::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_advanced_filter_creation() {
        let filter = AdvancedEmailFilter {
            id: Uuid::new_v4(),
            name: "Test Filter".to_string(),
            description: "A test filter".to_string(),
            enabled: true,
            priority: 1,
            condition_group: ConditionGroup {
                logic: BooleanLogic::And,
                conditions: vec![],
                nested_groups: vec![],
            },
            action_rules: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
            author: "test".to_string(),
            tags: vec![],
            statistics: FilterStatistics::default(),
        };

        assert_eq!(filter.name, "Test Filter");
        assert!(filter.enabled);
    }

    #[tokio::test]
    async fn test_filter_engine_creation() {
        let engine = AdvancedFilterEngine::new();
        assert_eq!(engine.filters.len(), 0);
        assert!(!engine.templates.templates.is_empty());
    }

    #[tokio::test]
    async fn test_template_library() {
        let library = FilterTemplateLibrary::new();
        assert!(library.get_template("spam_filter").is_some());
        assert!(library.get_template("newsletter_organizer").is_some());
        assert!(library.get_template("important_clients").is_some());
        assert!(library.get_template("weekend_backup").is_some());
    }
}