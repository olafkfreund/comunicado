use chrono::Utc;
use std::collections::HashMap;
use tokio::sync::RwLock;

use crate::calendar::database::CalendarDatabase;
use crate::calendar::event::AttendeeStatus;
use crate::calendar::invitation::{
    InvitationError, InvitationProcessor, InvitationResult, MeetingInvitation, RSVPResponse,
};
use crate::calendar::CalendarError;
use crate::email::StoredMessage;
use crate::smtp::SmtpService;

/// Meeting invitation manager for processing invitations and handling RSVP responses
pub struct InvitationManager {
    /// Processor for extracting invitations from emails
    processor: InvitationProcessor,

    /// Calendar database for storing accepted events
    database: CalendarDatabase,

    /// SMTP service for sending RSVP responses
    smtp_service: Option<std::sync::Arc<SmtpService>>,

    /// Cache of processed invitations
    invitation_cache: RwLock<HashMap<String, MeetingInvitation>>,

    /// User's primary calendar ID
    primary_calendar_id: String,

    /// User's email addresses
    user_emails: Vec<String>,
}

impl InvitationManager {
    /// Create a new invitation manager
    pub fn new(
        user_emails: Vec<String>,
        database: CalendarDatabase,
        smtp_service: Option<std::sync::Arc<SmtpService>>,
        primary_calendar_id: String,
    ) -> Self {
        let processor = InvitationProcessor::new(user_emails.clone());

        Self {
            processor,
            database,
            smtp_service,
            invitation_cache: RwLock::new(HashMap::new()),
            primary_calendar_id,
            user_emails,
        }
    }

    /// Check if an email contains a meeting invitation
    pub fn has_invitation(&self, message: &StoredMessage) -> bool {
        self.processor.has_invitation(message)
    }

    /// Process a meeting invitation from an email message
    pub async fn process_invitation(
        &self,
        message: &StoredMessage,
    ) -> InvitationResult<Option<MeetingInvitation>> {
        let invitation_opt = self.processor.extract_invitation(message).await?;

        if let Some(invitation) = invitation_opt {
            // Cache the invitation
            let mut cache = self.invitation_cache.write().await;
            cache.insert(invitation.uid.clone(), invitation.clone());

            Ok(Some(invitation))
        } else {
            Ok(None)
        }
    }

    /// Get a cached invitation by UID
    pub async fn get_invitation(&self, uid: &str) -> Option<MeetingInvitation> {
        let cache = self.invitation_cache.read().await;
        cache.get(uid).cloned()
    }

    /// Handle RSVP response to a meeting invitation
    pub async fn respond_to_invitation(
        &self,
        invitation_uid: &str,
        response: RSVPResponse,
        comment: Option<String>,
    ) -> InvitationResult<()> {
        let invitation = {
            let cache = self.invitation_cache.read().await;
            cache.get(invitation_uid).cloned().ok_or_else(|| {
                InvitationError::EmailError("Invitation not found in cache".to_string())
            })?
        };

        // Update user's status in the invitation
        let mut updated_invitation = invitation.clone();
        let user_email = self.get_user_email_in_invitation(&invitation)?;

        // Find and update user's attendee status
        for attendee in &mut updated_invitation.attendees {
            if attendee.email.eq_ignore_ascii_case(&user_email) {
                attendee.status = response.to_attendee_status();
                attendee.updated_at = Utc::now();
                break;
            }
        }

        // If accepted or tentative, add to calendar
        match response {
            RSVPResponse::Accept | RSVPResponse::Tentative => {
                let event = self
                    .processor
                    .invitation_to_event(&updated_invitation, self.primary_calendar_id.clone());
                self.database.store_event(&event).await.map_err(|e| {
                    InvitationError::CalendarError(CalendarError::DatabaseError(format!(
                        "Failed to store event: {}",
                        e
                    )))
                })?;
            }
            RSVPResponse::Decline => {
                // Remove from calendar if it was previously accepted
                if let Ok(existing_event) = self.database.get_event(&invitation.uid).await {
                    if existing_event.is_some() {
                        self.database
                            .delete_event(&invitation.uid)
                            .await
                            .map_err(|e| {
                                InvitationError::CalendarError(CalendarError::DatabaseError(
                                    format!("Failed to remove event: {}", e),
                                ))
                            })?;
                    }
                }
            }
            RSVPResponse::NeedsAction => {
                // No action needed for this state
            }
        }

        // Send RSVP email response if SMTP is configured
        if let Some(smtp_service) = &self.smtp_service {
            if let Err(e) = self
                .send_rsvp_email_response(
                    smtp_service,
                    &invitation,
                    &user_email,
                    response,
                    comment.clone(),
                )
                .await
            {
                tracing::error!("Failed to send RSVP email response: {}", e);
                // Don't fail the whole operation if email sending fails
            }
        } else {
            tracing::info!("SMTP service not configured, skipping RSVP email response");
        }

        // Update cache
        let mut cache = self.invitation_cache.write().await;
        cache.insert(invitation_uid.to_string(), updated_invitation);

        Ok(())
    }

    /// Add invitation to calendar without responding
    pub async fn add_to_calendar(&self, invitation_uid: &str) -> InvitationResult<()> {
        let invitation = {
            let cache = self.invitation_cache.read().await;
            cache.get(invitation_uid).cloned().ok_or_else(|| {
                InvitationError::EmailError("Invitation not found in cache".to_string())
            })?
        };

        let event = self
            .processor
            .invitation_to_event(&invitation, self.primary_calendar_id.clone());
        self.database.store_event(&event).await.map_err(|e| {
            InvitationError::CalendarError(CalendarError::DatabaseError(format!(
                "Failed to store event: {}",
                e
            )))
        })?;

        Ok(())
    }

    /// Get user's current status for an invitation
    pub async fn get_user_status(&self, invitation_uid: &str) -> Option<AttendeeStatus> {
        let cache = self.invitation_cache.read().await;
        if let Some(invitation) = cache.get(invitation_uid) {
            self.processor.get_user_rsvp_status(invitation)
        } else {
            None
        }
    }

    /// Check if user is invited to a meeting
    pub async fn is_user_invited(&self, invitation_uid: &str) -> bool {
        let cache = self.invitation_cache.read().await;
        if let Some(invitation) = cache.get(invitation_uid) {
            self.processor.is_user_invited(invitation)
        } else {
            false
        }
    }

    /// Get all cached invitations
    pub async fn get_all_invitations(&self) -> Vec<MeetingInvitation> {
        let cache = self.invitation_cache.read().await;
        cache.values().cloned().collect()
    }

    /// Clear invitation cache
    pub async fn clear_cache(&self) {
        let mut cache = self.invitation_cache.write().await;
        cache.clear();
    }

    /// Get user's email address that appears in the invitation
    fn get_user_email_in_invitation(
        &self,
        invitation: &MeetingInvitation,
    ) -> InvitationResult<String> {
        for attendee in &invitation.attendees {
            for user_email in &self.user_emails {
                if attendee.email.eq_ignore_ascii_case(user_email) {
                    return Ok(attendee.email.clone());
                }
            }
        }

        Err(InvitationError::EmailError(
            "User not found in invitation attendees".to_string(),
        ))
    }

    /// Send RSVP response email
    async fn send_rsvp_email_response(
        &self,
        smtp_service: &SmtpService,
        invitation: &MeetingInvitation,
        user_email: &str,
        response: RSVPResponse,
        comment: Option<String>,
    ) -> InvitationResult<()> {
        // Convert RSVPResponse to string format expected by SMTP service
        let response_str = match response {
            RSVPResponse::Accept => "ACCEPTED",
            RSVPResponse::Decline => "DECLINED",
            RSVPResponse::Tentative => "TENTATIVE",
            RSVPResponse::NeedsAction => "NEEDS-ACTION",
        };

        // Try to find which account this user email belongs to
        let account_id = self.find_account_for_email(user_email).await;
        if account_id.is_none() {
            tracing::warn!(
                "No account configured for email {}, cannot send RSVP",
                user_email
            );
            return Err(InvitationError::EmailError(format!(
                "No account configured for email: {}",
                user_email
            )));
        }
        let account_id = account_id.unwrap();

        // Get organizer email, handle Option<EventAttendee>
        let organizer_email = invitation
            .organizer
            .as_ref()
            .map(|org| &org.email)
            .ok_or_else(|| {
                InvitationError::EmailError("No organizer found in invitation".to_string())
            })?;

        // Send RSVP response via SMTP
        match smtp_service
            .send_rsvp_response(
                &account_id,
                user_email,
                organizer_email,
                &invitation.title,
                &invitation.uid,
                response_str,
                comment,
                &invitation.icalendar_data, // Include original iCalendar data
            )
            .await
        {
            Ok(send_result) => {
                if send_result.is_success() {
                    tracing::info!(
                        "RSVP response sent successfully for invitation: {}",
                        invitation.uid
                    );
                    Ok(())
                } else {
                    let error_msg = format!("RSVP response failed: {:?}", send_result);
                    tracing::error!("{}", error_msg);
                    Err(InvitationError::EmailError(error_msg))
                }
            }
            Err(e) => {
                let error_msg = format!("Failed to send RSVP response: {}", e);
                tracing::error!("{}", error_msg);
                Err(InvitationError::EmailError(error_msg))
            }
        }
    }

    /// Find which account ID corresponds to a user email address
    async fn find_account_for_email(&self, email: &str) -> Option<String> {
        // For now, use a simple approach - if SMTP service is configured for any account
        // that matches this email domain or the email itself, use that account
        // This is a simplified implementation - in practice, we'd want more sophisticated
        // account mapping

        if let Some(smtp_service) = &self.smtp_service {
            let accounts = smtp_service.get_configured_accounts().await;
            for account_id in &accounts {
                // Simple heuristic: if the account ID contains the email or domain, use it
                if account_id.contains(email) || email.contains(account_id) {
                    return Some(account_id.clone());
                }
            }
            // If no exact match, return the first available account
            // This is not ideal but provides a fallback
            if !accounts.is_empty() {
                tracing::warn!(
                    "No exact account match for email {}, using first available account: {}",
                    email,
                    accounts[0]
                );
                return Some(accounts[0].clone());
            }
        }
        None
    }

    /// Check for invitation updates in emails
    pub async fn check_for_updates(
        &self,
        messages: &[StoredMessage],
    ) -> InvitationResult<Vec<String>> {
        let mut updated_invitations = Vec::new();

        for message in messages {
            if let Some(invitation) = self.process_invitation(message).await? {
                // Check if this is an update to an existing invitation
                let cache = self.invitation_cache.read().await;
                if let Some(existing) = cache.get(&invitation.uid) {
                    if invitation.sequence > existing.sequence {
                        updated_invitations.push(invitation.uid.clone());
                    }
                }
            }
        }

        Ok(updated_invitations)
    }

    /// Get invitation statistics
    pub async fn get_statistics(&self) -> InvitationStatistics {
        let cache = self.invitation_cache.read().await;
        let mut stats = InvitationStatistics::default();

        for invitation in cache.values() {
            stats.total_invitations += 1;

            match invitation.method {
                crate::calendar::invitation::InvitationMethod::Request => {
                    if self.processor.is_user_invited(invitation) {
                        match self.processor.get_user_rsvp_status(invitation) {
                            Some(AttendeeStatus::Accepted) => stats.accepted += 1,
                            Some(AttendeeStatus::Declined) => stats.declined += 1,
                            Some(AttendeeStatus::Tentative) => stats.tentative += 1,
                            _ => stats.pending += 1,
                        }
                    }
                }
                crate::calendar::invitation::InvitationMethod::Cancel => stats.cancelled += 1,
                _ => {}
            }
        }

        stats
    }
}

/// Statistics for meeting invitations
#[derive(Debug, Default)]
pub struct InvitationStatistics {
    pub total_invitations: usize,
    pub accepted: usize,
    pub declined: usize,
    pub tentative: usize,
    pub pending: usize,
    pub cancelled: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calendar::database::CalendarDatabase;

    #[tokio::test]
    async fn test_invitation_manager_creation() {
        let db = CalendarDatabase::new_in_memory().await.unwrap();
        let user_emails = vec!["test@example.com".to_string()];
        let manager = InvitationManager::new(user_emails, db, None, "test-calendar".to_string());

        let stats = manager.get_statistics().await;
        assert_eq!(stats.total_invitations, 0);
    }

    #[test]
    fn test_ical_reply_creation() {
        let _db = CalendarDatabase::new_in_memory();
        // This would need to be implemented as an async test
        // with proper invitation data to test the RSVP functionality
    }
}
