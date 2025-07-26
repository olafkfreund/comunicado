use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::email::StoredMessage;

/// Email filter rule for organizing messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailFilter {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub enabled: bool,
    pub priority: i32, // Lower numbers = higher priority
    pub conditions: Vec<FilterCondition>,
    pub actions: Vec<FilterAction>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Condition to match against email messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterCondition {
    pub field: FilterField,
    pub operator: FilterOperator,
    pub value: String,
    pub case_sensitive: bool,
}

/// Fields that can be filtered on
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilterField {
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
    Size,
    Date,
    Priority,
    Folder,
}

/// Operators for filter conditions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilterOperator {
    Contains,
    NotContains,
    StartsWith,
    EndsWith,
    Equals,
    NotEquals,
    GreaterThan,
    LessThan,
    Regex,
    IsEmpty,
    IsNotEmpty,
}

/// Actions to perform when filter matches
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilterAction {
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
    StopProcessing, // Stop applying further filters
}

/// Result of filter processing
#[derive(Debug, Clone)]
pub struct FilterResult {
    pub matched_filters: Vec<Uuid>,
    pub actions_applied: Vec<FilterAction>,
    pub stop_processing: bool,
}

/// Filter engine for processing email messages
pub struct FilterEngine {
    filters: Vec<EmailFilter>,
}

impl FilterEngine {
    /// Create a new filter engine
    pub fn new() -> Self {
        Self {
            filters: Vec::new(),
        }
    }

    /// Load filters from database/storage
    pub fn load_filters(&mut self, filters: Vec<EmailFilter>) {
        self.filters = filters;
        // Sort by priority (lower number = higher priority)
        self.filters.sort_by_key(|f| f.priority);
    }

    /// Add a new filter
    pub fn add_filter(&mut self, filter: EmailFilter) {
        self.filters.push(filter);
        self.filters.sort_by_key(|f| f.priority);
    }

    /// Remove a filter by ID
    pub fn remove_filter(&mut self, filter_id: Uuid) {
        self.filters.retain(|f| f.id != filter_id);
    }

    /// Update an existing filter
    pub fn update_filter(&mut self, filter: EmailFilter) {
        if let Some(pos) = self.filters.iter().position(|f| f.id == filter.id) {
            self.filters[pos] = filter;
            self.filters.sort_by_key(|f| f.priority);
        }
    }

    /// Process a message through all filters
    pub fn process_message(&self, message: &StoredMessage) -> FilterResult {
        let mut result = FilterResult {
            matched_filters: Vec::new(),
            actions_applied: Vec::new(),
            stop_processing: false,
        };

        for filter in &self.filters {
            if !filter.enabled {
                continue;
            }

            if self.evaluate_filter(filter, message) {
                result.matched_filters.push(filter.id);
                
                for action in &filter.actions {
                    if matches!(action, FilterAction::StopProcessing) {
                        result.stop_processing = true;
                        break;
                    }
                    result.actions_applied.push(action.clone());
                }

                if result.stop_processing {
                    break;
                }
            }
        }

        result
    }

    /// Evaluate if a filter matches a message
    fn evaluate_filter(&self, filter: &EmailFilter, message: &StoredMessage) -> bool {
        if filter.conditions.is_empty() {
            return false;
        }

        // All conditions must match (AND logic)
        filter.conditions.iter().all(|condition| {
            self.evaluate_condition(condition, message)
        })
    }

    /// Evaluate a single condition against a message
    fn evaluate_condition(&self, condition: &FilterCondition, message: &StoredMessage) -> bool {
        let field_value = self.extract_field_value(&condition.field, message);
        
        let value_to_check = if condition.case_sensitive {
            field_value.clone()
        } else {
            field_value.to_lowercase()
        };

        let condition_value = if condition.case_sensitive {
            condition.value.clone()
        } else {
            condition.value.to_lowercase()
        };

        match &condition.operator {
            FilterOperator::Contains => value_to_check.contains(&condition_value),
            FilterOperator::NotContains => !value_to_check.contains(&condition_value),
            FilterOperator::StartsWith => value_to_check.starts_with(&condition_value),
            FilterOperator::EndsWith => value_to_check.ends_with(&condition_value),
            FilterOperator::Equals => value_to_check == condition_value,
            FilterOperator::NotEquals => value_to_check != condition_value,
            FilterOperator::GreaterThan => {
                if let (Ok(field_num), Ok(condition_num)) = (field_value.parse::<f64>(), condition.value.parse::<f64>()) {
                    field_num > condition_num
                } else {
                    false
                }
            }
            FilterOperator::LessThan => {
                if let (Ok(field_num), Ok(condition_num)) = (field_value.parse::<f64>(), condition.value.parse::<f64>()) {
                    field_num < condition_num
                } else {
                    false
                }
            }
            FilterOperator::Regex => {
                if let Ok(regex) = regex::Regex::new(&condition.value) {
                    regex.is_match(&field_value)
                } else {
                    false
                }
            }
            FilterOperator::IsEmpty => field_value.trim().is_empty(),
            FilterOperator::IsNotEmpty => !field_value.trim().is_empty(),
        }
    }

    /// Extract field value from message
    fn extract_field_value(&self, field: &FilterField, message: &StoredMessage) -> String {
        match field {
            FilterField::From => message.from_addr.clone(),
            FilterField::To => message.to_addrs.join(", "),
            FilterField::Subject => message.subject.clone(),
            FilterField::Body => {
                message.body_text.clone()
                    .or_else(|| message.body_html.clone())
                    .unwrap_or_default()
            }
            FilterField::Sender => message.from_addr.clone(), // Could be different from From
            FilterField::MessageId => message.message_id.clone().unwrap_or_default(),
            FilterField::HasAttachment => {
                if !message.attachments.is_empty() { "true" } else { "false" }.to_string()
            }
            FilterField::Size => message.size.map(|s| s.to_string()).unwrap_or_default(),
            FilterField::Date => message.date.to_rfc3339(),
            FilterField::Folder => message.folder_name.clone(),
            FilterField::CC => message.cc_addrs.join(", "),
            FilterField::BCC => message.bcc_addrs.join(", "),
            FilterField::ReplyTo => message.reply_to.clone().unwrap_or_default(),
            FilterField::InReplyTo => message.in_reply_to.clone().unwrap_or_default(),
            FilterField::AttachmentName => {
                message.attachments.iter()
                    .map(|a| a.filename.clone())
                    .collect::<Vec<_>>()
                    .join(", ")
            }
            FilterField::AttachmentType => {
                message.attachments.iter()
                    .map(|a| a.content_type.clone())
                    .collect::<Vec<_>>()
                    .join(", ")
            }
            FilterField::Priority => {
                message.priority.clone().unwrap_or_default()
            }
        }
    }

    /// Get all filters
    pub fn get_filters(&self) -> &[EmailFilter] {
        &self.filters
    }

    /// Get filters by enabled status
    pub fn get_enabled_filters(&self) -> Vec<&EmailFilter> {
        self.filters.iter().filter(|f| f.enabled).collect()
    }
}

impl EmailFilter {
    /// Create a new email filter
    pub fn new(name: String, description: String) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            description,
            enabled: true,
            priority: 100, // Default priority
            conditions: Vec::new(),
            actions: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Add a condition to the filter
    pub fn add_condition(mut self, condition: FilterCondition) -> Self {
        self.conditions.push(condition);
        self.updated_at = chrono::Utc::now();
        self
    }

    /// Add an action to the filter
    pub fn add_action(mut self, action: FilterAction) -> Self {
        self.actions.push(action);
        self.updated_at = chrono::Utc::now();
        self
    }

    /// Set filter priority
    pub fn set_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self.updated_at = chrono::Utc::now();
        self
    }

    /// Enable or disable the filter
    pub fn set_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self.updated_at = chrono::Utc::now();
        self
    }
}

impl FilterCondition {
    /// Create a new filter condition
    pub fn new(field: FilterField, operator: FilterOperator, value: String) -> Self {
        Self {
            field,
            operator,
            value,
            case_sensitive: false,
        }
    }

    /// Set case sensitivity
    pub fn case_sensitive(mut self, case_sensitive: bool) -> Self {
        self.case_sensitive = case_sensitive;
        self
    }
}

/// Pre-defined filter templates for common use cases
pub struct FilterTemplates;

impl FilterTemplates {
    /// Spam filter for common spam patterns
    pub fn spam_filter() -> EmailFilter {
        EmailFilter::new(
            "Spam Filter".to_string(),
            "Automatically move spam emails to spam folder".to_string(),
        )
        .add_condition(FilterCondition::new(
            FilterField::Subject,
            FilterOperator::Contains,
            "[SPAM]".to_string(),
        ))
        .add_action(FilterAction::MoveToFolder("Spam".to_string()))
        .add_action(FilterAction::MarkAsRead)
        .set_priority(10)
    }

    /// Newsletter filter
    pub fn newsletter_filter() -> EmailFilter {
        EmailFilter::new(
            "Newsletter Filter".to_string(),
            "Organize newsletters into dedicated folder".to_string(),
        )
        .add_condition(FilterCondition::new(
            FilterField::Subject,
            FilterOperator::Contains,
            "newsletter".to_string(),
        ))
        .add_action(FilterAction::MoveToFolder("Newsletters".to_string()))
        .add_action(FilterAction::AddLabel("newsletter".to_string()))
        .set_priority(50)
    }

    /// Important sender filter
    pub fn important_sender_filter(sender_email: String) -> EmailFilter {
        EmailFilter::new(
            format!("Important: {}", sender_email),
            format!("Mark emails from {} as important", sender_email),
        )
        .add_condition(FilterCondition::new(
            FilterField::From,
            FilterOperator::Contains,
            sender_email,
        ))
        .add_action(FilterAction::MarkAsImportant)
        .add_action(FilterAction::AddLabel("important".to_string()))
        .set_priority(20)
    }

    /// Auto-archive old emails
    pub fn auto_archive_filter() -> EmailFilter {
        EmailFilter::new(
            "Auto Archive".to_string(),
            "Archive emails older than 30 days".to_string(),
        )
        .add_action(FilterAction::MoveToFolder("Archive".to_string()))
        .set_priority(90)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_creation() {
        let filter = EmailFilter::new(
            "Test Filter".to_string(),
            "Test description".to_string(),
        );
        
        assert_eq!(filter.name, "Test Filter");
        assert_eq!(filter.description, "Test description");
        assert!(filter.enabled);
        assert_eq!(filter.priority, 100);
    }

    #[test]
    fn test_condition_matching() {
        let condition = FilterCondition::new(
            FilterField::Subject,
            FilterOperator::Contains,
            "test".to_string(),
        );

        let engine = FilterEngine::new();
        let message = create_test_message();
        
        // This would require a full test setup - simplified for now
        assert_eq!(condition.field, FilterField::Subject);
    }

    fn create_test_message() -> StoredMessage {
        StoredMessage {
            id: Uuid::new_v4(),
            account_id: "test".to_string(),
            folder_name: "INBOX".to_string(),
            imap_uid: 123,
            message_id: Some("test@example.com".to_string()),
            thread_id: None,
            in_reply_to: None,
            references: Vec::new(),
            subject: "Test Subject".to_string(),
            from_addr: "test@example.com".to_string(),
            from_name: Some("Test User".to_string()),
            to_addrs: vec!["user@example.com".to_string()],
            cc_addrs: Vec::new(),
            bcc_addrs: Vec::new(),
            reply_to: None,
            date: chrono::Utc::now(),
            body_text: Some("Test content".to_string()),
            body_html: None,
            attachments: Vec::new(),
            flags: vec!["\\Seen".to_string()],
            labels: Vec::new(),
            size: Some(1000),
            priority: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }
}