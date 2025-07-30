//! Advanced search functionality for contacts
//!
//! This module provides sophisticated search capabilities including multi-criteria
//! search, fuzzy matching, search history, and advanced filtering options.

use crate::contacts::{
    Contact, ContactSearchCriteria, ContactSource, ContactsDatabase, ContactsError, ContactsResult,
};
use sqlx::Row;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Advanced search criteria with multiple filters and options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedSearchCriteria {
    /// General query text (searches across all fields)
    pub query: Option<String>,
    
    /// Specific field searches
    pub name_query: Option<String>,
    pub email_query: Option<String>,
    pub phone_query: Option<String>,
    pub company_query: Option<String>,
    pub notes_query: Option<String>,
    
    /// Contact source filters
    pub sources: Vec<ContactSource>,
    
    /// Email and phone presence filters
    pub has_email: Option<bool>,
    pub has_phone: Option<bool>,
    pub has_company: Option<bool>,
    pub has_notes: Option<bool>,
    
    /// Date range filters
    pub created_after: Option<DateTime<Utc>>,
    pub created_before: Option<DateTime<Utc>>,
    pub updated_after: Option<DateTime<Utc>>,
    pub updated_before: Option<DateTime<Utc>>,
    pub synced_after: Option<DateTime<Utc>>,
    pub synced_before: Option<DateTime<Utc>>,
    
    /// Search options
    pub fuzzy_matching: bool,
    pub case_sensitive: bool,
    pub whole_word_only: bool,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    
    /// Sorting options
    pub sort_field: SortField,
    pub sort_direction: SortDirection,
}

/// Fields available for sorting search results
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SortField {
    DisplayName,
    FirstName,
    LastName,
    Company,
    Email,
    Phone,
    CreatedAt,
    UpdatedAt,
    SyncedAt,
    Relevance, // For fuzzy search ranking
}

/// Sort direction
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SortDirection {
    Ascending,
    Descending,
}

/// Search result with relevance scoring
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub contact: Contact,
    pub relevance_score: f64,
    pub matching_fields: Vec<String>,
    pub snippets: HashMap<String, String>, // Field -> highlighted snippet
}

/// Saved search for quick access
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedSearch {
    pub id: Option<i64>,
    pub name: String,
    pub description: Option<String>,
    pub criteria: AdvancedSearchCriteria,
    pub created_at: DateTime<Utc>,
    pub last_used: Option<DateTime<Utc>>,
    pub use_count: u32,
}

/// Search history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHistoryEntry {
    pub id: Option<i64>,
    pub query_text: String,
    pub criteria: AdvancedSearchCriteria,
    pub result_count: usize,
    pub executed_at: DateTime<Utc>,
    pub execution_time_ms: u64,
}

/// Advanced search engine for contacts
pub struct AdvancedContactSearch {
    database: ContactsDatabase,
}

impl Default for AdvancedSearchCriteria {
    fn default() -> Self {
        Self {
            query: None,
            name_query: None,
            email_query: None,
            phone_query: None,
            company_query: None,
            notes_query: None,
            sources: Vec::new(),
            has_email: None,
            has_phone: None,
            has_company: None,
            has_notes: None,
            created_after: None,
            created_before: None,
            updated_after: None,
            updated_before: None,
            synced_after: None,
            synced_before: None,
            fuzzy_matching: false,
            case_sensitive: false,
            whole_word_only: false,
            limit: Some(50),
            offset: None,
            sort_field: SortField::DisplayName,
            sort_direction: SortDirection::Ascending,
        }
    }
}

impl AdvancedSearchCriteria {
    /// Create new search criteria
    pub fn new() -> Self {
        Self::default()
    }

    /// Set general query
    pub fn with_query(mut self, query: String) -> Self {
        self.query = Some(query);
        self
    }

    /// Set name-specific query
    pub fn with_name_query(mut self, query: String) -> Self {
        self.name_query = Some(query);
        self
    }

    /// Set email-specific query
    pub fn with_email_query(mut self, query: String) -> Self {
        self.email_query = Some(query);
        self
    }

    /// Set phone-specific query
    pub fn with_phone_query(mut self, query: String) -> Self {
        self.phone_query = Some(query);
        self
    }

    /// Set company-specific query
    pub fn with_company_query(mut self, query: String) -> Self {
        self.company_query = Some(query);
        self
    }

    /// Filter by contact sources
    pub fn with_sources(mut self, sources: Vec<ContactSource>) -> Self {
        self.sources = sources;
        self
    }

    /// Filter by email presence
    pub fn with_email_filter(mut self, has_email: bool) -> Self {
        self.has_email = Some(has_email);
        self
    }

    /// Filter by phone presence
    pub fn with_phone_filter(mut self, has_phone: bool) -> Self {
        self.has_phone = Some(has_phone);
        self
    }

    /// Enable fuzzy matching
    pub fn with_fuzzy_matching(mut self, enabled: bool) -> Self {
        self.fuzzy_matching = enabled;
        self
    }

    /// Set sorting options
    pub fn with_sort(mut self, field: SortField, direction: SortDirection) -> Self {
        self.sort_field = field;
        self.sort_direction = direction;
        self
    }

    /// Set result limit
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Check if any search criteria are set
    pub fn is_empty(&self) -> bool {
        self.query.is_none()
            && self.name_query.is_none()
            && self.email_query.is_none()
            && self.phone_query.is_none()
            && self.company_query.is_none()
            && self.notes_query.is_none()
            && self.sources.is_empty()
            && self.has_email.is_none()
            && self.has_phone.is_none()
            && self.has_company.is_none()
            && self.has_notes.is_none()
            && self.created_after.is_none()
            && self.created_before.is_none()
            && self.updated_after.is_none()
            && self.updated_before.is_none()
            && self.synced_after.is_none()
            && self.synced_before.is_none()
    }

    /// Convert to basic search criteria for fallback
    pub fn to_basic_criteria(&self) -> ContactSearchCriteria {
        let mut criteria = ContactSearchCriteria::new();
        
        if let Some(query) = &self.query {
            criteria = criteria.with_query(query.clone());
        }
        
        if let Some(email) = &self.email_query {
            criteria = criteria.with_email(email.clone());
        }
        
        if let Some(limit) = self.limit {
            criteria = criteria.with_limit(limit);
        }
        
        if let Some(source) = self.sources.first() {
            criteria = criteria.with_source(source.clone());
        }
        
        criteria
    }
}

impl AdvancedContactSearch {
    /// Create new advanced search engine
    pub async fn new(database_url: &str) -> ContactsResult<Self> {
        let database = ContactsDatabase::new(database_url).await?;
        Ok(Self { database })
    }

    /// Perform advanced search with scoring and ranking
    pub async fn search(&self, criteria: &AdvancedSearchCriteria) -> ContactsResult<Vec<SearchResult>> {
        let start_time = std::time::Instant::now();
        
        // Build optimized SQL query based on criteria
        let (query, params) = self.build_search_query(criteria)?;
        
        // Execute query
        let rows = sqlx::query(&query)
            .bind_all(params)
            .fetch_all(&self.database.pool)
            .await
            .map_err(|e| ContactsError::DatabaseError(e.to_string()))?;
        
        // Convert rows to contacts
        let mut results = Vec::new();
        for row in rows {
            let contact = self.database.contact_from_row(&row)?;
            let relevance_score = self.calculate_relevance_score(&contact, criteria);
            let matching_fields = self.find_matching_fields(&contact, criteria);
            let snippets = self.generate_snippets(&contact, criteria);
            
            results.push(SearchResult {
                contact,
                relevance_score,
                matching_fields,
                snippets,
            });
        }
        
        // Sort results based on criteria
        self.sort_results(&mut results, criteria);
        
        // Record search in history
        let execution_time = start_time.elapsed().as_millis() as u64;
        let _ = self.add_to_search_history(criteria, results.len(), execution_time).await;
        
        Ok(results)
    }

    /// Build optimized SQL query from search criteria
    fn build_search_query(&self, criteria: &AdvancedSearchCriteria) -> ContactsResult<(String, Vec<String>)> {
        let mut query = "SELECT c.*, 
                               GROUP_CONCAT(DISTINCT e.address || '|' || e.label || '|' || e.is_primary) as emails,
                               GROUP_CONCAT(DISTINCT p.number || '|' || p.label || '|' || p.is_primary) as phones
                        FROM contacts c
                        LEFT JOIN contact_emails e ON c.id = e.contact_id
                        LEFT JOIN contact_phones p ON c.id = p.contact_id
                        WHERE 1=1".to_string();

        let mut params = Vec::new();
        let mut param_count = 1;

        // General query search across multiple fields
        if let Some(general_query) = &criteria.query {
            let pattern = if criteria.fuzzy_matching {
                format!("%{}%", general_query)
            } else if criteria.case_sensitive {
                general_query.clone()
            } else {
                format!("%{}%", general_query.to_lowercase())
            };

            if criteria.case_sensitive {
                query.push_str(&format!(
                    " AND (c.display_name LIKE ?{} OR c.first_name LIKE ?{} OR c.last_name LIKE ?{} OR c.company LIKE ?{} OR c.notes LIKE ?{})",
                    param_count, param_count + 1, param_count + 2, param_count + 3, param_count + 4
                ));
            } else {
                query.push_str(&format!(
                    " AND (LOWER(c.display_name) LIKE ?{} OR LOWER(c.first_name) LIKE ?{} OR LOWER(c.last_name) LIKE ?{} OR LOWER(c.company) LIKE ?{} OR LOWER(c.notes) LIKE ?{})",
                    param_count, param_count + 1, param_count + 2, param_count + 3, param_count + 4
                ));
            }
            
            params.extend(vec![pattern.clone(); 5]);
            param_count += 5;
        }

        // Specific field searches
        if let Some(name_query) = &criteria.name_query {
            let pattern = self.build_search_pattern(name_query, criteria);
            if criteria.case_sensitive {
                query.push_str(&format!(" AND (c.display_name LIKE ?{} OR c.first_name LIKE ?{} OR c.last_name LIKE ?{})", param_count, param_count + 1, param_count + 2));
            } else {
                query.push_str(&format!(" AND (LOWER(c.display_name) LIKE ?{} OR LOWER(c.first_name) LIKE ?{} OR LOWER(c.last_name) LIKE ?{})", param_count, param_count + 1, param_count + 2));
            }
            params.extend(vec![pattern.clone(); 3]);
            param_count += 3;
        }

        if let Some(email_query) = &criteria.email_query {
            let pattern = self.build_search_pattern(email_query, criteria);
            query.push_str(&format!(" AND c.id IN (SELECT contact_id FROM contact_emails WHERE {} address LIKE ?{})", 
                if criteria.case_sensitive { "" } else { "LOWER(" }, param_count));
            if !criteria.case_sensitive {
                query.push_str(")");
            }
            params.push(pattern);
            param_count += 1;
        }

        if let Some(phone_query) = &criteria.phone_query {
            let pattern = self.build_search_pattern(phone_query, criteria);
            query.push_str(&format!(" AND c.id IN (SELECT contact_id FROM contact_phones WHERE number LIKE ?{})", param_count));
            params.push(pattern);
            param_count += 1;
        }

        if let Some(company_query) = &criteria.company_query {
            let pattern = self.build_search_pattern(company_query, criteria);
            if criteria.case_sensitive {
                query.push_str(&format!(" AND c.company LIKE ?{}", param_count));
            } else {
                query.push_str(&format!(" AND LOWER(c.company) LIKE ?{}", param_count));
            }
            params.push(pattern);
            param_count += 1;
        }

        if let Some(notes_query) = &criteria.notes_query {
            let pattern = self.build_search_pattern(notes_query, criteria);
            if criteria.case_sensitive {
                query.push_str(&format!(" AND c.notes LIKE ?{}", param_count));
            } else {
                query.push_str(&format!(" AND LOWER(c.notes) LIKE ?{}", param_count));
            }
            params.push(pattern);
            param_count += 1;
        }

        // Source filters
        if !criteria.sources.is_empty() {
            let mut source_conditions = Vec::new();
            for source in &criteria.sources {
                match source {
                    ContactSource::Google { account_id } => {
                        source_conditions.push(format!("(c.source_type = 'google' AND c.source_account_id = ?{})", param_count));
                        params.push(account_id.clone());
                        param_count += 1;
                    }
                    ContactSource::Outlook { account_id } => {
                        source_conditions.push(format!("(c.source_type = 'outlook' AND c.source_account_id = ?{})", param_count));
                        params.push(account_id.clone());
                        param_count += 1;
                    }
                    ContactSource::Local => {
                        source_conditions.push("c.source_type = 'local'".to_string());
                    }
                }
            }
            if !source_conditions.is_empty() {
                query.push_str(&format!(" AND ({})", source_conditions.join(" OR ")));
            }
        }

        // Presence filters
        if let Some(has_email) = criteria.has_email {
            if has_email {
                query.push_str(" AND c.id IN (SELECT contact_id FROM contact_emails)");
            } else {
                query.push_str(" AND c.id NOT IN (SELECT contact_id FROM contact_emails)");
            }
        }

        if let Some(has_phone) = criteria.has_phone {
            if has_phone {
                query.push_str(" AND c.id IN (SELECT contact_id FROM contact_phones)");
            } else {
                query.push_str(" AND c.id NOT IN (SELECT contact_id FROM contact_phones)");
            }
        }

        if let Some(has_company) = criteria.has_company {
            if has_company {
                query.push_str(" AND c.company IS NOT NULL AND c.company != ''");
            } else {
                query.push_str(" AND (c.company IS NULL OR c.company = '')");
            }
        }

        if let Some(has_notes) = criteria.has_notes {
            if has_notes {
                query.push_str(" AND c.notes IS NOT NULL AND c.notes != ''");
            } else {
                query.push_str(" AND (c.notes IS NULL OR c.notes = '')");
            }
        }

        // Date filters
        if let Some(created_after) = criteria.created_after {
            query.push_str(&format!(" AND c.created_at >= ?{}", param_count));
            params.push(created_after.to_rfc3339());
            param_count += 1;
        }

        if let Some(created_before) = criteria.created_before {
            query.push_str(&format!(" AND c.created_at <= ?{}", param_count));
            params.push(created_before.to_rfc3339());
            param_count += 1;
        }

        if let Some(updated_after) = criteria.updated_after {
            query.push_str(&format!(" AND c.updated_at >= ?{}", param_count));
            params.push(updated_after.to_rfc3339());
            param_count += 1;
        }

        if let Some(updated_before) = criteria.updated_before {
            query.push_str(&format!(" AND c.updated_at <= ?{}", param_count));
            params.push(updated_before.to_rfc3339());
            param_count += 1;
        }

        if let Some(synced_after) = criteria.synced_after {
            query.push_str(&format!(" AND c.synced_at >= ?{}", param_count));
            params.push(synced_after.to_rfc3339());
            param_count += 1;
        }

        if let Some(synced_before) = criteria.synced_before {
            query.push_str(&format!(" AND c.synced_at <= ?{}", param_count));
            params.push(synced_before.to_rfc3339());
            param_count += 1;
        }

        // Group by and sorting
        query.push_str(" GROUP BY c.id");
        
        let sort_clause = self.build_sort_clause(criteria);
        query.push_str(&sort_clause);

        // Limit and offset
        if let Some(limit) = criteria.limit {
            query.push_str(&format!(" LIMIT {}", limit));
            if let Some(offset) = criteria.offset {
                query.push_str(&format!(" OFFSET {}", offset));
            }
        }

        Ok((query, params))
    }

    /// Build search pattern based on criteria options
    fn build_search_pattern(&self, query: &str, criteria: &AdvancedSearchCriteria) -> String {
        if criteria.whole_word_only {
            // Use word boundaries for whole word matching
            format!("% {} %", query)
        } else if criteria.case_sensitive {
            format!("%{}%", query)
        } else {
            format!("%{}%", query.to_lowercase())
        }
    }

    /// Build SQL sort clause
    fn build_sort_clause(&self, criteria: &AdvancedSearchCriteria) -> String {
        let direction = match criteria.sort_direction {
            SortDirection::Ascending => "ASC",
            SortDirection::Descending => "DESC",
        };

        let field = match criteria.sort_field {
            SortField::DisplayName => "c.display_name",
            SortField::FirstName => "c.first_name",
            SortField::LastName => "c.last_name",
            SortField::Company => "c.company",
            SortField::Email => "(SELECT e.address FROM contact_emails e WHERE e.contact_id = c.id AND e.is_primary = 1 LIMIT 1)",
            SortField::Phone => "(SELECT p.number FROM contact_phones p WHERE p.contact_id = c.id AND p.is_primary = 1 LIMIT 1)",
            SortField::CreatedAt => "c.created_at",
            SortField::UpdatedAt => "c.updated_at",
            SortField::SyncedAt => "c.synced_at",
            SortField::Relevance => "c.display_name", // Will be overridden by relevance scoring
        };

        format!(" ORDER BY {} {}", field, direction)
    }

    /// Calculate relevance score for search results
    fn calculate_relevance_score(&self, contact: &Contact, criteria: &AdvancedSearchCriteria) -> f64 {
        let mut score = 0.0;

        // Base score for exact matches in display name (highest priority)
        if let Some(query) = &criteria.query {
            if contact.display_name.to_lowercase().contains(&query.to_lowercase()) {
                score += 10.0;
                if contact.display_name.to_lowercase() == query.to_lowercase() {
                    score += 20.0; // Exact match bonus
                }
            }
        }

        // Score for name field matches
        if let Some(name_query) = &criteria.name_query {
            let name_text = format!("{} {} {}", 
                contact.display_name, 
                contact.first_name.as_deref().unwrap_or(""),
                contact.last_name.as_deref().unwrap_or("")
            ).to_lowercase();
            
            if name_text.contains(&name_query.to_lowercase()) {
                score += 8.0;
            }
        }

        // Score for email matches
        if let Some(email_query) = &criteria.email_query {
            for email in &contact.emails {
                if email.address.to_lowercase().contains(&email_query.to_lowercase()) {
                    score += 6.0;
                    if email.is_primary {
                        score += 2.0; // Primary email bonus
                    }
                }
            }
        }

        // Score for company matches
        if let Some(company_query) = &criteria.company_query {
            if let Some(company) = &contact.company {
                if company.to_lowercase().contains(&company_query.to_lowercase()) {
                    score += 5.0;
                }
            }
        }

        // Score for phone matches
        if let Some(phone_query) = &criteria.phone_query {
            for phone in &contact.phones {
                if phone.number.contains(phone_query) {
                    score += 4.0;
                    if phone.is_primary {
                        score += 1.0; // Primary phone bonus
                    }
                }
            }
        }

        // Bonus for recent activity
        let now = Utc::now();
        let days_since_update = (now - contact.updated_at).num_days();
        if days_since_update < 30 {
            score += 2.0 - (days_since_update as f64 / 15.0); // Decay over 30 days
        }

        // Bonus for having complete information
        if !contact.emails.is_empty() { score += 0.5; }
        if !contact.phones.is_empty() { score += 0.5; }
        if contact.company.is_some() { score += 0.3; }
        if contact.notes.is_some() { score += 0.2; }

        score.max(0.0)
    }

    /// Find which fields matched the search criteria
    fn find_matching_fields(&self, contact: &Contact, criteria: &AdvancedSearchCriteria) -> Vec<String> {
        let mut matching_fields = Vec::new();

        if let Some(query) = &criteria.query {
            let query_lower = query.to_lowercase();
            if contact.display_name.to_lowercase().contains(&query_lower) {
                matching_fields.push("display_name".to_string());
            }
            if let Some(first_name) = &contact.first_name {
                if first_name.to_lowercase().contains(&query_lower) {
                    matching_fields.push("first_name".to_string());
                }
            }
            if let Some(last_name) = &contact.last_name {
                if last_name.to_lowercase().contains(&query_lower) {
                    matching_fields.push("last_name".to_string());
                }
            }
            if let Some(company) = &contact.company {
                if company.to_lowercase().contains(&query_lower) {
                    matching_fields.push("company".to_string());
                }
            }
        }

        if let Some(email_query) = &criteria.email_query {
            for email in &contact.emails {
                if email.address.to_lowercase().contains(&email_query.to_lowercase()) {
                    matching_fields.push("email".to_string());
                    break;
                }
            }
        }

        if let Some(phone_query) = &criteria.phone_query {
            for phone in &contact.phones {
                if phone.number.contains(phone_query) {
                    matching_fields.push("phone".to_string());
                    break;
                }
            }
        }

        matching_fields
    }

    /// Generate highlighted snippets for matching fields
    fn generate_snippets(&self, contact: &Contact, criteria: &AdvancedSearchCriteria) -> HashMap<String, String> {
        let mut snippets = HashMap::new();

        if let Some(query) = &criteria.query {
            let query_lower = query.to_lowercase();
            
            // Generate snippet for display name if it matches
            if contact.display_name.to_lowercase().contains(&query_lower) {
                let highlighted = self.highlight_text(&contact.display_name, query);
                snippets.insert("display_name".to_string(), highlighted);
            }

            // Generate snippet for company if it matches
            if let Some(company) = &contact.company {
                if company.to_lowercase().contains(&query_lower) {
                    let highlighted = self.highlight_text(company, query);
                    snippets.insert("company".to_string(), highlighted);
                }
            }
        }

        snippets
    }

    /// Highlight matching text in a string
    fn highlight_text(&self, text: &str, query: &str) -> String {
        // Simple highlighting - in a real implementation, you might use regex or more sophisticated matching
        let lower_text = text.to_lowercase();
        let lower_query = query.to_lowercase();
        
        if let Some(start) = lower_text.find(&lower_query) {
            let end = start + query.len();
            format!("{}**{}**{}", 
                &text[..start],
                &text[start..end],
                &text[end..]
            )
        } else {
            text.to_string()
        }
    }

    /// Sort search results based on criteria
    fn sort_results(&self, results: &mut Vec<SearchResult>, criteria: &AdvancedSearchCriteria) {
        match criteria.sort_field {
            SortField::Relevance => {
                results.sort_by(|a, b| {
                    match criteria.sort_direction {
                        SortDirection::Ascending => a.relevance_score.partial_cmp(&b.relevance_score).unwrap_or(std::cmp::Ordering::Equal),
                        SortDirection::Descending => b.relevance_score.partial_cmp(&a.relevance_score).unwrap_or(std::cmp::Ordering::Equal),
                    }
                });
            }
            SortField::DisplayName => {
                results.sort_by(|a, b| {
                    match criteria.sort_direction {
                        SortDirection::Ascending => a.contact.display_name.cmp(&b.contact.display_name),
                        SortDirection::Descending => b.contact.display_name.cmp(&a.contact.display_name),
                    }
                });
            }
            SortField::Company => {
                results.sort_by(|a, b| {
                    let a_company = a.contact.company.as_deref().unwrap_or("");
                    let b_company = b.contact.company.as_deref().unwrap_or("");
                    match criteria.sort_direction {
                        SortDirection::Ascending => a_company.cmp(b_company),
                        SortDirection::Descending => b_company.cmp(a_company),
                    }
                });
            }
            SortField::CreatedAt => {
                results.sort_by(|a, b| {
                    match criteria.sort_direction {
                        SortDirection::Ascending => a.contact.created_at.cmp(&b.contact.created_at),
                        SortDirection::Descending => b.contact.created_at.cmp(&a.contact.created_at),
                    }
                });
            }
            SortField::UpdatedAt => {
                results.sort_by(|a, b| {
                    match criteria.sort_direction {
                        SortDirection::Ascending => a.contact.updated_at.cmp(&b.contact.updated_at),
                        SortDirection::Descending => b.contact.updated_at.cmp(&a.contact.updated_at),
                    }
                });
            }
            // For other fields, results are already sorted by SQL query
            _ => {}
        }
    }

    /// Add search to history
    async fn add_to_search_history(
        &self, 
        criteria: &AdvancedSearchCriteria, 
        result_count: usize, 
        execution_time_ms: u64
    ) -> ContactsResult<()> {
        let query_text = self.criteria_to_summary_text(criteria);
        
        // For now, we'll just log the search. In a full implementation, 
        // this would be stored in a search_history table
        tracing::info!(
            "Search executed: '{}' - {} results in {}ms", 
            query_text, 
            result_count, 
            execution_time_ms
        );
        
        Ok(())
    }

    /// Convert search criteria to human-readable summary
    fn criteria_to_summary_text(&self, criteria: &AdvancedSearchCriteria) -> String {
        let mut parts = Vec::new();

        if let Some(query) = &criteria.query {
            parts.push(format!("General: '{}'", query));
        }

        if let Some(name) = &criteria.name_query {
            parts.push(format!("Name: '{}'", name));
        }

        if let Some(email) = &criteria.email_query {
            parts.push(format!("Email: '{}'", email));
        }

        if let Some(company) = &criteria.company_query {
            parts.push(format!("Company: '{}'", company));
        }

        if !criteria.sources.is_empty() {
            let source_names: Vec<String> = criteria.sources.iter()
                .map(|s| s.provider_name().to_string())
                .collect();
            parts.push(format!("Sources: {}", source_names.join(", ")));
        }

        if parts.is_empty() {
            "Empty search".to_string()
        } else {
            parts.join(" | ")
        }
    }

    /// Get search suggestions based on contact data
    pub async fn get_search_suggestions(&self, partial_query: &str) -> ContactsResult<Vec<String>> {
        let mut suggestions = Vec::new();

        // Get common names, companies, and email domains
        let rows = sqlx::query("
            SELECT DISTINCT display_name FROM contacts 
            WHERE display_name LIKE ? 
            ORDER BY display_name LIMIT 5
        ")
        .bind(format!("{}%", partial_query))
        .fetch_all(&self.database.pool)
        .await
        .map_err(|e| ContactsError::DatabaseError(e.to_string()))?;

        for row in rows {
            if let Ok(name) = row.try_get::<String, _>("display_name") {
                suggestions.push(name);
            }
        }

        // Get company suggestions
        let company_rows = sqlx::query("
            SELECT DISTINCT company FROM contacts 
            WHERE company IS NOT NULL AND company LIKE ? 
            ORDER BY company LIMIT 3
        ")
        .bind(format!("{}%", partial_query))
        .fetch_all(&self.database.pool)
        .await
        .map_err(|e| ContactsError::DatabaseError(e.to_string()))?;

        for row in company_rows {
            if let Ok(company) = row.try_get::<String, _>("company") {
                suggestions.push(format!("Company: {}", company));
            }
        }

        Ok(suggestions)
    }
}

/// Extension trait for sqlx::QueryBuilder to handle dynamic parameter binding
trait QueryBuilderExt {
    fn bind_all(self, params: Vec<String>) -> Self;
}

impl<'q> QueryBuilderExt for sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'q>> {
    fn bind_all(mut self, params: Vec<String>) -> Self {
        for param in params {
            self = self.bind(param);
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_advanced_search_creation() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path().to_str().unwrap();
        
        let search_engine = AdvancedContactSearch::new(db_path).await;
        assert!(search_engine.is_ok());
    }

    #[tokio::test]
    async fn test_search_criteria_builder() {
        let criteria = AdvancedSearchCriteria::new()
            .with_query("John Doe".to_string())
            .with_fuzzy_matching(true)
            .with_sort(SortField::Relevance, SortDirection::Descending);

        assert_eq!(criteria.query, Some("John Doe".to_string()));
        assert!(criteria.fuzzy_matching);
        assert_eq!(criteria.sort_field, SortField::Relevance);
        assert_eq!(criteria.sort_direction, SortDirection::Descending);
    }

    #[tokio::test]
    async fn test_criteria_conversion() {
        let advanced_criteria = AdvancedSearchCriteria::new()
            .with_query("test".to_string())
            .with_email_query("example@test.com".to_string())
            .with_limit(25);

        let basic_criteria = advanced_criteria.to_basic_criteria();
        assert_eq!(basic_criteria.query, Some("test".to_string()));
        assert_eq!(basic_criteria.email, Some("example@test.com".to_string()));
        assert_eq!(basic_criteria.limit, Some(25));
    }

    #[tokio::test]
    async fn test_empty_criteria_check() {
        let empty_criteria = AdvancedSearchCriteria::new();
        assert!(empty_criteria.is_empty());

        let non_empty_criteria = AdvancedSearchCriteria::new()
            .with_query("test".to_string());
        assert!(!non_empty_criteria.is_empty());
    }
}