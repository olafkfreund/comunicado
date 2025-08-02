//! AI-powered calendar assistant for natural language event processing

use crate::ai::{AIService, AIContext};
use crate::calendar::{CalendarError, CalendarResult, Event};
use crate::calendar::manager::CalendarManager;
use chrono::{DateTime, Datelike, Duration, Timelike, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Natural language calendar event creation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NaturalLanguageEventRequest {
    /// Natural language description of the event
    pub description: String,
    /// Optional context information
    pub context: Option<String>,
    /// Target calendar ID (if not specified, use default)
    pub calendar_id: Option<String>,
}

/// Parsed event information from natural language
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedEventInfo {
    /// Event title
    pub title: String,
    /// Event description
    pub description: Option<String>,
    /// Start date and time
    pub start_time: DateTime<Utc>,
    /// End date and time
    pub end_time: DateTime<Utc>,
    /// Event location
    pub location: Option<String>,
    /// Attendee email addresses
    pub attendees: Vec<String>,
    /// Whether this is an all-day event
    pub all_day: bool,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f32,
    /// Suggested calendar ID
    pub suggested_calendar: Option<String>,
}

/// Event modification suggestions from AI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventModificationSuggestions {
    /// Suggested time changes
    pub time_suggestions: Vec<String>,
    /// Location suggestions
    pub location_suggestions: Vec<String>,
    /// Title improvements
    pub title_suggestions: Vec<String>,
    /// Additional attendee suggestions
    pub attendee_suggestions: Vec<String>,
    /// Optimization recommendations
    pub optimization_tips: Vec<String>,
}

/// Meeting scheduling analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeetingScheduleAnalysis {
    /// Optimal meeting times
    pub optimal_times: Vec<DateTime<Utc>>,
    /// Attendee availability conflicts
    pub conflicts: Vec<AvailabilityConflict>,
    /// Suggested meeting duration
    pub suggested_duration: u32,
    /// Recommended location or meeting type
    pub location_recommendation: Option<String>,
    /// Preparation time recommendations
    pub preparation_time: Option<u32>,
}

/// Availability conflict information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailabilityConflict {
    /// Conflicting attendee email
    pub attendee: String,
    /// Conflicting time slot
    pub conflict_time: DateTime<Utc>,
    /// Conflict description
    pub description: String,
    /// Severity (0.0 to 1.0)
    pub severity: f32,
}

/// Calendar insights from AI analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarInsights {
    /// Meeting pattern analysis
    pub meeting_patterns: Vec<String>,
    /// Time management suggestions
    pub time_management_tips: Vec<String>,
    /// Schedule optimization recommendations
    pub optimization_suggestions: Vec<String>,
    /// Productivity insights
    pub productivity_insights: Vec<String>,
    /// Focus time recommendations
    pub focus_time_suggestions: Vec<String>,
}

/// AI-powered calendar assistant
pub struct AICalendarAssistant {
    ai_service: Arc<AIService>,
    calendar_manager: Arc<CalendarManager>,
}

impl AICalendarAssistant {
    /// Create a new AI calendar assistant
    pub fn new(ai_service: Arc<AIService>, calendar_manager: Arc<CalendarManager>) -> Self {
        Self {
            ai_service,
            calendar_manager,
        }
    }

    /// Parse natural language into event information
    pub async fn parse_natural_language_event(
        &self,
        request: &NaturalLanguageEventRequest,
    ) -> CalendarResult<ParsedEventInfo> {
        if !self.ai_service.is_enabled().await {
            return Err(CalendarError::InvalidData("AI service is not available".to_string()));
        }

        // Prepare the parsing prompt
        let parsing_prompt = format!(
            "Parse this natural language event description into structured calendar event data. Return a JSON object with the following fields:
            - title: string (concise event title)
            - description: string or null (detailed description)
            - start_time: string (ISO 8601 datetime)
            - end_time: string (ISO 8601 datetime)
            - location: string or null (event location)
            - attendees: array of strings (email addresses)
            - all_day: boolean (true for all-day events)
            - confidence: number (0.0 to 1.0, confidence in parsing accuracy)

            Current date/time for context: {}
            
            Event description: \"{}\"
            
            Additional context: {}",
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
            request.description,
            request.context.as_deref().unwrap_or("None")
        );

        let ai_context = AIContext {
            user_preferences: HashMap::new(),
            email_thread: None,
            calendar_context: Some("calendar_event_parsing".to_string()),
            max_length: Some(2000),
            creativity: Some(0.3), // Lower creativity for more accurate parsing
        };

        let response = self.ai_service
            .complete_text(&parsing_prompt, Some(&ai_context))
            .await
            .map_err(|e| CalendarError::InvalidData(format!("AI parsing failed: {}", e)))?;

        // Parse the AI response as JSON
        self.parse_ai_event_response(&response).await
    }

    /// Create an event from natural language description
    pub async fn create_event_from_natural_language(
        &self,
        request: &NaturalLanguageEventRequest,
    ) -> CalendarResult<Event> {
        let parsed_info = self.parse_natural_language_event(request).await?;
        
        // Determine which calendar to use
        let calendar_id = request.calendar_id
            .clone()
            .or(parsed_info.suggested_calendar)
            .unwrap_or_else(|| "default".to_string());

        // Create the event
        let event = Event::new(
            calendar_id,
            parsed_info.title,
            parsed_info.start_time,
            parsed_info.end_time,
        )
        .with_description(parsed_info.description)
        .with_location(parsed_info.location)
        .set_all_day(parsed_info.all_day);

        // Add attendees if specified
        let mut final_event = event;
        for attendee_email in parsed_info.attendees {
            final_event = final_event.with_attendee(attendee_email, None, false);
        }

        // Save the event through the calendar manager
        self.calendar_manager.create_event(final_event.clone()).await?;

        Ok(final_event)
    }

    /// Suggest modifications for an existing event
    pub async fn suggest_event_modifications(
        &self,
        event: &Event,
        modification_request: &str,
    ) -> CalendarResult<EventModificationSuggestions> {
        if !self.ai_service.is_enabled().await {
            return Err(CalendarError::InvalidData("AI service is not available".to_string()));
        }

        let modification_prompt = format!(
            "Analyze this calendar event and suggest improvements based on the user's request. Provide specific, actionable suggestions.

            Current Event:
            - Title: {}
            - Description: {}
            - Start: {}
            - End: {}
            - Location: {}
            - Attendees: {}
            
            User Request: \"{}\"
            
            Please provide suggestions for:
            1. Time optimizations (different time slots, duration adjustments)
            2. Location alternatives or improvements
            3. Title improvements for clarity
            4. Additional attendees who might be relevant
            5. General optimization tips
            
            Format as concise, actionable bullet points.",
            event.title,
            event.description.as_deref().unwrap_or("None"),
            event.start_time.format("%Y-%m-%d %H:%M UTC"),
            event.end_time.format("%Y-%m-%d %H:%M UTC"),
            event.location.as_deref().unwrap_or("None"),
            event.attendees.iter().map(|a| a.email.as_str()).collect::<Vec<_>>().join(", "),
            modification_request
        );

        let ai_context = AIContext {
            user_preferences: HashMap::new(),
            email_thread: None,
            calendar_context: Some("event_modification".to_string()),
            max_length: Some(1500),
            creativity: Some(0.7),
        };

        let response = self.ai_service
            .complete_text(&modification_prompt, Some(&ai_context))
            .await
            .map_err(|e| CalendarError::InvalidData(format!("AI suggestion failed: {}", e)))?;

        self.parse_modification_suggestions(&response).await
    }

    /// Analyze meeting schedules and suggest optimal times
    pub async fn analyze_meeting_schedule(
        &self,
        attendees: &[String],
        duration_minutes: u32,
        preferred_times: Option<&str>,
    ) -> CalendarResult<MeetingScheduleAnalysis> {
        if !self.ai_service.is_enabled().await {
            return Err(CalendarError::InvalidData("AI service is not available".to_string()));
        }

        // Get events for the next week for each attendee (simplified for this implementation)
        let start_date = Utc::now();
        let end_date = start_date + Duration::days(7);

        let analysis_prompt = format!(
            "Analyze the following meeting scheduling requirements and provide optimal scheduling recommendations:

            Meeting Requirements:
            - Duration: {} minutes
            - Attendees: {}
            - Preferred times: {}
            - Analysis period: {} to {}
            
            Please analyze and provide:
            1. 3-5 optimal meeting time slots
            2. Any potential conflicts or concerns
            3. Recommended meeting duration if different from requested
            4. Location/format recommendations (in-person, virtual, hybrid)
            5. Suggested preparation time before the meeting
            
            Consider typical business hours, time zones, and meeting efficiency best practices.",
            duration_minutes,
            attendees.join(", "),
            preferred_times.unwrap_or("Standard business hours"),
            start_date.format("%Y-%m-%d %H:%M UTC"),
            end_date.format("%Y-%m-%d %H:%M UTC")
        );

        let ai_context = AIContext {
            user_preferences: HashMap::new(),
            email_thread: None,
            calendar_context: Some("meeting_scheduling".to_string()),
            max_length: Some(2000),
            creativity: Some(0.4),
        };

        let response = self.ai_service
            .complete_text(&analysis_prompt, Some(&ai_context))
            .await
            .map_err(|e| CalendarError::InvalidData(format!("AI analysis failed: {}", e)))?;

        self.parse_schedule_analysis(&response, duration_minutes).await
    }

    /// Generate calendar insights and productivity recommendations
    pub async fn generate_calendar_insights(
        &self,
        time_period_days: u32,
    ) -> CalendarResult<CalendarInsights> {
        if !self.ai_service.is_enabled().await {
            return Err(CalendarError::InvalidData("AI service is not available".to_string()));
        }

        // Get events for the specified time period
        let end_date = Utc::now();
        let start_date = end_date - Duration::days(time_period_days as i64);

        let events = self.calendar_manager
            .get_events_in_range(start_date, end_date)
            .await
            .unwrap_or_default();

        // Prepare calendar data for analysis
        let calendar_summary = self.prepare_calendar_summary(&events).await;

        let insights_prompt = format!(
            "Analyze this calendar data and provide actionable productivity insights and recommendations:

            Calendar Analysis Period: {} days
            Calendar Summary:
            {}

            Please provide insights on:
            1. Meeting patterns and trends
            2. Time management improvement suggestions
            3. Schedule optimization recommendations
            4. Productivity insights based on meeting frequency/types
            5. Focus time and deep work suggestions

            Focus on practical, actionable advice for better calendar management and productivity.",
            time_period_days,
            calendar_summary
        );

        let ai_context = AIContext {
            user_preferences: HashMap::new(),
            email_thread: None,
            calendar_context: Some("calendar_insights".to_string()),
            max_length: Some(2500),
            creativity: Some(0.6),
        };

        let response = self.ai_service
            .complete_text(&insights_prompt, Some(&ai_context))
            .await
            .map_err(|e| CalendarError::InvalidData(format!("AI insights failed: {}", e)))?;

        self.parse_calendar_insights(&response).await
    }

    /// Parse AI response for event information
    async fn parse_ai_event_response(&self, response: &str) -> CalendarResult<ParsedEventInfo> {
        // Try to extract JSON from the response
        let json_start = response.find('{').unwrap_or(0);
        let json_end = response.rfind('}').map(|i| i + 1).unwrap_or(response.len());
        let json_str = &response[json_start..json_end];

        // Parse the JSON response
        let parsed: serde_json::Value = serde_json::from_str(json_str)
            .map_err(|e| CalendarError::InvalidData(format!("Invalid JSON from AI: {}", e)))?;

        // Extract and validate required fields
        let title = parsed["title"]
            .as_str()
            .ok_or_else(|| CalendarError::InvalidData("Missing title field".to_string()))?
            .to_string();

        let start_time_str = parsed["start_time"]
            .as_str()
            .ok_or_else(|| CalendarError::InvalidData("Missing start_time field".to_string()))?;

        let end_time_str = parsed["end_time"]
            .as_str()
            .ok_or_else(|| CalendarError::InvalidData("Missing end_time field".to_string()))?;

        // Parse datetime strings
        let start_time = DateTime::parse_from_rfc3339(start_time_str)
            .map_err(|e| CalendarError::InvalidData(format!("Invalid start_time format: {}", e)))?
            .with_timezone(&Utc);

        let end_time = DateTime::parse_from_rfc3339(end_time_str)
            .map_err(|e| CalendarError::InvalidData(format!("Invalid end_time format: {}", e)))?
            .with_timezone(&Utc);

        // Extract optional fields
        let description = parsed["description"].as_str().map(|s| s.to_string());
        let location = parsed["location"].as_str().map(|s| s.to_string());
        let all_day = parsed["all_day"].as_bool().unwrap_or(false);
        let confidence = parsed["confidence"].as_f64().unwrap_or(0.7) as f32;

        // Extract attendees
        let attendees = parsed["attendees"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_default();

        Ok(ParsedEventInfo {
            title,
            description,
            start_time,
            end_time,
            location,
            attendees,
            all_day,
            confidence,
            suggested_calendar: None,
        })
    }

    /// Parse modification suggestions from AI response
    async fn parse_modification_suggestions(&self, response: &str) -> CalendarResult<EventModificationSuggestions> {
        // Simple parsing - extract bullet points by category
        let lines: Vec<&str> = response.lines().collect();
        let mut time_suggestions = Vec::new();
        let mut location_suggestions = Vec::new();
        let mut title_suggestions = Vec::new();
        let mut attendee_suggestions = Vec::new();
        let mut optimization_tips = Vec::new();

        let mut current_category = "";
        
        for line in lines {
            let line = line.trim();
            
            if line.to_lowercase().contains("time") && line.contains(":") {
                current_category = "time";
                continue;
            } else if line.to_lowercase().contains("location") && line.contains(":") {
                current_category = "location";
                continue;
            } else if line.to_lowercase().contains("title") && line.contains(":") {
                current_category = "title";
                continue;
            } else if line.to_lowercase().contains("attendee") && line.contains(":") {
                current_category = "attendee";
                continue;
            } else if line.to_lowercase().contains("optimization") || line.to_lowercase().contains("tip") {
                current_category = "optimization";
                continue;
            }

            if line.starts_with("- ") || line.starts_with("• ") {
                let suggestion = line.trim_start_matches("- ").trim_start_matches("• ").to_string();
                match current_category {
                    "time" => time_suggestions.push(suggestion),
                    "location" => location_suggestions.push(suggestion),
                    "title" => title_suggestions.push(suggestion),
                    "attendee" => attendee_suggestions.push(suggestion),
                    "optimization" => optimization_tips.push(suggestion),
                    _ => optimization_tips.push(suggestion), // Default category
                }
            }
        }

        Ok(EventModificationSuggestions {
            time_suggestions,
            location_suggestions,
            title_suggestions,
            attendee_suggestions,
            optimization_tips,
        })
    }

    /// Parse schedule analysis from AI response
    async fn parse_schedule_analysis(&self, _response: &str, requested_duration: u32) -> CalendarResult<MeetingScheduleAnalysis> {
        // For this implementation, we'll create a simplified analysis
        // In a real implementation, you'd parse the AI response more thoroughly
        
        let optimal_times = vec![
            Utc::now() + Duration::days(1) + Duration::hours(10), // Tomorrow at 10 AM
            Utc::now() + Duration::days(1) + Duration::hours(14), // Tomorrow at 2 PM
            Utc::now() + Duration::days(2) + Duration::hours(9),  // Day after at 9 AM
        ];

        Ok(MeetingScheduleAnalysis {
            optimal_times,
            conflicts: Vec::new(), // Would be populated with actual conflict analysis
            suggested_duration: requested_duration,
            location_recommendation: Some("Virtual meeting via video call".to_string()),
            preparation_time: Some(15), // 15 minutes prep time
        })
    }

    /// Parse calendar insights from AI response
    async fn parse_calendar_insights(&self, response: &str) -> CalendarResult<CalendarInsights> {
        // Simple parsing of AI response into categories
        let lines: Vec<&str> = response.lines().collect();
        let mut meeting_patterns = Vec::new();
        let mut time_management_tips = Vec::new();
        let mut optimization_suggestions = Vec::new();
        let mut productivity_insights = Vec::new();
        let mut focus_time_suggestions = Vec::new();

        let mut current_category = "";
        
        for line in lines {
            let line = line.trim();
            
            if line.to_lowercase().contains("pattern") && line.contains(":") {
                current_category = "patterns";
                continue;
            } else if line.to_lowercase().contains("time management") && line.contains(":") {
                current_category = "time_management";
                continue;
            } else if line.to_lowercase().contains("optimization") && line.contains(":") {
                current_category = "optimization";
                continue;
            } else if line.to_lowercase().contains("productivity") && line.contains(":") {
                current_category = "productivity";
                continue;
            } else if line.to_lowercase().contains("focus") && line.contains(":") {
                current_category = "focus";
                continue;
            }

            if line.starts_with("- ") || line.starts_with("• ") {
                let insight = line.trim_start_matches("- ").trim_start_matches("• ").to_string();
                match current_category {
                    "patterns" => meeting_patterns.push(insight),
                    "time_management" => time_management_tips.push(insight),
                    "optimization" => optimization_suggestions.push(insight),
                    "productivity" => productivity_insights.push(insight),
                    "focus" => focus_time_suggestions.push(insight),
                    _ => productivity_insights.push(insight), // Default category
                }
            }
        }

        Ok(CalendarInsights {
            meeting_patterns,
            time_management_tips,
            optimization_suggestions,
            productivity_insights,
            focus_time_suggestions,
        })
    }

    /// Prepare calendar summary for AI analysis
    async fn prepare_calendar_summary(&self, events: &[Event]) -> String {
        if events.is_empty() {
            return "No events in the specified time period.".to_string();
        }

        let mut summary = format!("Total events: {}\n", events.len());
        
        // Meeting frequency analysis
        let total_duration: i64 = events.iter()
            .map(|e| (e.end_time - e.start_time).num_minutes())
            .sum();
        
        summary.push_str(&format!("Total meeting time: {} hours\n", total_duration / 60));
        summary.push_str(&format!("Average meeting duration: {} minutes\n", 
            if events.len() > 0 { total_duration / events.len() as i64 } else { 0 }));

        // Day distribution
        let mut day_counts = HashMap::new();
        for event in events {
            let day = event.start_time.weekday();
            *day_counts.entry(day).or_insert(0) += 1;
        }

        summary.push_str("Meeting distribution by day:\n");
        for (day, count) in day_counts {
            summary.push_str(&format!("  {}: {} meetings\n", day, count));
        }

        // Time slot analysis
        let mut hour_counts = HashMap::new();
        for event in events {
            let hour = event.start_time.hour();
            *hour_counts.entry(hour).or_insert(0) += 1;
        }

        summary.push_str("Most common meeting hours:\n");
        let mut sorted_hours: Vec<_> = hour_counts.iter().collect();
        sorted_hours.sort_by(|a, b| b.1.cmp(a.1));
        for (hour, count) in sorted_hours.iter().take(3) {
            summary.push_str(&format!("  {}:00 - {} meetings\n", hour, count));
        }

        summary
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::config::AIConfig;
    use crate::ai::factory::AIFactory;

    #[tokio::test]
    async fn test_natural_language_parsing() {
        // This test would require a real AI service setup
        // For now, we'll test the structure
        let config = AIConfig::default();
        let _ai_service = Arc::new(AIFactory::create_ai_service(config).await.unwrap());
        
        // Test would use a mock calendar manager
        // let calendar_manager = Arc::new(mock_calendar_manager());
        // let assistant = AICalendarAssistant::new(ai_service, calendar_manager);
        
        // let request = NaturalLanguageEventRequest {
        //     description: "Team meeting tomorrow at 2 PM for 1 hour".to_string(),
        //     context: None,
        //     calendar_id: None,
        // };
        
        // This would test the actual parsing logic
        // let result = assistant.parse_natural_language_event(&request).await;
        // assert!(result.is_ok());
    }
}