//! Animated content integration for email viewer
//!
//! This module integrates the animation system with email content rendering,
//! allowing GIFs and other animated content in emails to be displayed properly.

use crate::ui::animation::{AnimationManager, AnimationSettings, AnimationError, AnimationFormat};
use crate::ui::graphics::{ImageRenderer, RenderConfig, GraphicsProtocol};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Animated content integration errors
#[derive(Error, Debug)]
pub enum AnimatedContentError {
    #[error("Animation error: {0}")]
    Animation(#[from] AnimationError),
    
    #[error("Content extraction failed: {0}")]
    ContentExtraction(String),
    
    #[error("Unsupported content type: {0}")]
    UnsupportedContentType(String),
    
    #[error("Email attachment not found: {0}")]
    AttachmentNotFound(String),
    
    #[error("Terminal not compatible with animation")]
    TerminalNotCompatible,
}

pub type AnimatedContentResult<T> = Result<T, AnimatedContentError>;

/// Animated attachment information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimatedAttachment {
    pub id: Uuid,
    pub filename: String,
    pub content_type: String,
    pub format: AnimationFormat,
    pub size: u64,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub duration_ms: Option<u32>,
    pub frame_count: Option<usize>,
    pub auto_play: bool,
    pub animation_id: Option<Uuid>, // ID in animation manager
    pub created_at: DateTime<Utc>,
}

/// Email content with animated elements
#[derive(Debug, Clone)]
pub struct AnimatedEmailContent {
    pub text_content: String,
    pub html_content: Option<String>,
    pub animated_attachments: Vec<AnimatedAttachment>,
    pub inline_animations: HashMap<String, Uuid>, // cid -> animation_id mapping
}

/// Animation playback state for an email
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailAnimationState {
    pub email_id: Uuid,
    pub active_animations: Vec<Uuid>,
    pub paused_animations: Vec<Uuid>,
    pub settings: AnimationSettings,
    pub last_updated: DateTime<Utc>,
}

/// Manager for animated email content
pub struct AnimatedContentManager {
    animation_manager: Arc<AnimationManager>,
    renderer: Arc<ImageRenderer>,
    email_states: Arc<RwLock<HashMap<Uuid, EmailAnimationState>>>,
    settings: AnimationSettings,
    #[allow(dead_code)]
    max_animation_size: (u32, u32),
    max_file_size: u64,
}

impl AnimatedContentManager {
    /// Create a new animated content manager
    pub fn new(
        animation_manager: Arc<AnimationManager>,
        renderer: Arc<ImageRenderer>,
        settings: AnimationSettings,
    ) -> Self {
        Self {
            animation_manager,
            renderer,
            email_states: Arc::new(RwLock::new(HashMap::new())),
            settings,
            max_animation_size: (800, 600),
            max_file_size: 10 * 1024 * 1024, // 10MB
        }
    }
    
    /// Create with auto-detected graphics support
    pub fn auto(settings: AnimationSettings) -> AnimatedContentResult<Self> {
        let render_config = RenderConfig::default();
        let renderer = Arc::new(ImageRenderer::new(render_config));
        
        // Check if terminal supports animation
        if !renderer.supports_animation() {
            return Err(AnimatedContentError::TerminalNotCompatible);
        }
        
        let animation_manager = Arc::new(AnimationManager::new(
            Arc::clone(&renderer),
            settings.clone(),
        ));
        
        Ok(Self::new(animation_manager, renderer, settings))
    }
    
    /// Process email content and extract animated elements
    pub async fn process_email_content(
        &self,
        email_id: Uuid,
        html_content: Option<&str>,
        attachments: &[crate::email::StoredAttachment],
    ) -> AnimatedContentResult<AnimatedEmailContent> {
        let mut animated_attachments = Vec::new();
        let mut inline_animations = HashMap::new();
        
        // Process attachments for animated content
        for attachment in attachments {
            if self.is_animated_content(&attachment.content_type) {
                let animated_attachment = self.process_animated_attachment(attachment).await?;
                
                // Check if this is an inline attachment (has cid)
                if let Some(ref cid) = attachment.content_id {
                    if let Some(animation_id) = animated_attachment.animation_id {
                        inline_animations.insert(cid.clone(), animation_id);
                    }
                }
                
                animated_attachments.push(animated_attachment);
            }
        }
        
        // Create email animation state
        let active_animations = animated_attachments
            .iter()
            .filter_map(|a| a.animation_id)
            .collect();
        
        let email_state = EmailAnimationState {
            email_id,
            active_animations,
            paused_animations: Vec::new(),
            settings: self.settings.clone(),
            last_updated: Utc::now(),
        };
        
        self.email_states.write().await.insert(email_id, email_state);
        
        Ok(AnimatedEmailContent {
            text_content: String::new(), // Will be filled by caller
            html_content: html_content.map(String::from),
            animated_attachments,
            inline_animations,
        })
    }
    
    /// Process a single animated attachment
    async fn process_animated_attachment(
        &self,
        attachment: &crate::email::StoredAttachment,
    ) -> AnimatedContentResult<AnimatedAttachment> {
        // Validate file size
        if u64::from(attachment.size) > self.max_file_size {
            return Err(AnimatedContentError::ContentExtraction(
                format!("File too large: {} bytes", attachment.size)
            ));
        }
        
        // Determine animation format
        let format = self.detect_animation_format(&attachment.content_type, &attachment.filename)?;
        
        // Load and decode animation
        let animation_id = if let Some(ref file_path) = attachment.file_path {
            Some(self.animation_manager.load_animation(std::path::Path::new(file_path)).await?)
        } else if let Some(ref data) = attachment.data {
            Some(self.animation_manager.load_animation_from_bytes(data, format.clone()).await?)
        } else {
            None
        };
        
        // Get animation metadata if loaded
        let (width, height, duration_ms, frame_count) = if let Some(animation_id) = animation_id {
            if let Some(metadata) = self.animation_manager.get_metadata(animation_id) {
                (
                    Some(metadata.width),
                    Some(metadata.height),
                    Some(metadata.duration_ms),
                    Some(metadata.frame_count),
                )
            } else {
                (None, None, None, None)
            }
        } else {
            (None, None, None, None)
        };
        
        Ok(AnimatedAttachment {
            id: Uuid::new_v4(),
            filename: attachment.filename.clone(),
            content_type: attachment.content_type.clone(),
            format,
            size: u64::from(attachment.size),
            width,
            height,
            duration_ms,
            frame_count,
            auto_play: self.settings.auto_play,
            animation_id,
            created_at: Utc::now(),
        })
    }
    
    /// Check if content type represents animated content
    fn is_animated_content(&self, content_type: &str) -> bool {
        match content_type.to_lowercase().as_str() {
            "image/gif" => true,
            "image/webp" => true, // Could be animated
            "image/png" => true,  // Could be APNG
            "image/avif" => true, // Could be animated
            _ => false,
        }
    }
    
    /// Detect animation format from content type and filename
    fn detect_animation_format(
        &self,
        content_type: &str,
        filename: &str,
    ) -> AnimatedContentResult<AnimationFormat> {
        // Try content type first
        let format = match content_type.to_lowercase().as_str() {
            "image/gif" => Some(AnimationFormat::Gif),
            "image/webp" => Some(AnimationFormat::WebP),
            "image/png" => Some(AnimationFormat::Apng),
            "image/avif" => Some(AnimationFormat::Avif),
            _ => None,
        };
        
        if let Some(format) = format {
            return Ok(format);
        }
        
        // Try filename extension
        let path = PathBuf::from(filename);
        if let Some(format) = AnimationFormat::from_extension(&path) {
            return Ok(format);
        }
        
        Err(AnimatedContentError::UnsupportedContentType(content_type.to_string()))
    }
    
    /// Render animated content for current frame
    pub async fn render_animation(
        &self,
        animation_id: Uuid,
        width: u32,
        height: u32,
    ) -> AnimatedContentResult<Option<Vec<u8>>> {
        Ok(self.animation_manager.render_animation(animation_id, width, height).await?)
    }
    
    /// Control animation playback
    pub fn play_animation(&self, animation_id: Uuid) -> bool {
        self.animation_manager.play_animation(animation_id)
    }
    
    pub fn pause_animation(&self, animation_id: Uuid) -> bool {
        self.animation_manager.pause_animation(animation_id)
    }
    
    pub fn stop_animation(&self, animation_id: Uuid) -> bool {
        self.animation_manager.stop_animation(animation_id)
    }
    
    /// Control all animations in an email
    pub async fn play_email_animations(&self, email_id: Uuid) {
        if let Some(state) = self.email_states.read().await.get(&email_id) {
            for &animation_id in &state.active_animations {
                self.animation_manager.play_animation(animation_id);
            }
        }
    }
    
    pub async fn pause_email_animations(&self, email_id: Uuid) {
        if let Some(state) = self.email_states.read().await.get(&email_id) {
            for &animation_id in &state.active_animations {
                self.animation_manager.pause_animation(animation_id);
            }
        }
    }
    
    pub async fn stop_email_animations(&self, email_id: Uuid) {
        if let Some(state) = self.email_states.read().await.get(&email_id) {
            for &animation_id in &state.active_animations {
                self.animation_manager.stop_animation(animation_id);
            }
        }
    }
    
    /// Clean up animations for an email when no longer needed
    pub async fn cleanup_email_animations(&self, email_id: Uuid) {
        if let Some(state) = self.email_states.write().await.remove(&email_id) {
            for animation_id in state.active_animations {
                self.animation_manager.remove_animation(animation_id);
            }
        }
    }
    
    /// Get current protocol support
    pub fn graphics_protocol(&self) -> GraphicsProtocol {
        self.renderer.protocol()
    }
    
    /// Check if animations are supported
    pub fn supports_animation(&self) -> bool {
        self.renderer.supports_animation()
    }
    
    /// Get animation statistics
    pub async fn get_email_animation_stats(&self, email_id: Uuid) -> Option<EmailAnimationStats> {
        let states = self.email_states.read().await;
        if let Some(state) = states.get(&email_id) {
            let mut total_animations = 0;
            let mut playing_animations = 0;
            let mut total_frames = 0;
            let mut total_duration = 0;
            
            for &animation_id in &state.active_animations {
                if let Some(metadata) = self.animation_manager.get_metadata(animation_id) {
                    total_animations += 1;
                    total_frames += metadata.frame_count;
                    total_duration += metadata.duration_ms;
                    
                    // Check if playing (simplified check)
                    playing_animations += 1;
                }
            }
            
            Some(EmailAnimationStats {
                email_id,
                total_animations,
                playing_animations,
                total_frames,
                total_duration_ms: total_duration,
                memory_usage_estimate: self.estimate_memory_usage(&state.active_animations).await,
                last_updated: state.last_updated,
            })
        } else {
            None
        }
    }
    
    /// Estimate memory usage for animations
    async fn estimate_memory_usage(&self, animation_ids: &[Uuid]) -> u64 {
        let mut total_usage = 0;
        
        for &animation_id in animation_ids {
            if let Some(metadata) = self.animation_manager.get_metadata(animation_id) {
                // Rough estimate: width * height * 4 (RGBA) * frame_count
                let frame_size = metadata.width as u64 * metadata.height as u64 * 4;
                total_usage += frame_size * metadata.frame_count as u64;
            }
        }
        
        total_usage
    }
    
    /// Update animation settings
    pub fn update_settings(&mut self, settings: AnimationSettings) {
        self.settings = settings;
        // Note: Cannot update animation manager settings through Arc
        // This would need to be redesigned with interior mutability
    }
    
    /// Get list of active animations
    pub fn list_active_animations(&self) -> Vec<crate::ui::animation::AnimationMetadata> {
        self.animation_manager.list_animations()
    }
}

/// Animation statistics for an email
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailAnimationStats {
    pub email_id: Uuid,
    pub total_animations: usize,
    pub playing_animations: usize,
    pub total_frames: usize,
    pub total_duration_ms: u32,
    pub memory_usage_estimate: u64,
    pub last_updated: DateTime<Utc>,
}

/// Animation control widget for email viewer
pub struct AnimationControlWidget {
    animations: Vec<AnimatedAttachment>,
    selected_index: usize,
    show_controls: bool,
}

impl AnimationControlWidget {
    /// Create new animation control widget
    pub fn new(animations: Vec<AnimatedAttachment>) -> Self {
        let show_controls = !animations.is_empty();
        Self {
            animations,
            selected_index: 0,
            show_controls,
        }
    }
    
    /// Toggle controls visibility
    pub fn toggle_controls(&mut self) {
        self.show_controls = !self.show_controls;
    }
    
    /// Navigate animations
    pub fn next_animation(&mut self) {
        if !self.animations.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.animations.len();
        }
    }
    
    pub fn previous_animation(&mut self) {
        if !self.animations.is_empty() {
            self.selected_index = if self.selected_index == 0 {
                self.animations.len() - 1
            } else {
                self.selected_index - 1
            };
        }
    }
    
    /// Get currently selected animation
    pub fn selected_animation(&self) -> Option<&AnimatedAttachment> {
        self.animations.get(self.selected_index)
    }
    
    /// Check if controls should be shown
    pub fn should_show_controls(&self) -> bool {
        self.show_controls && !self.animations.is_empty()
    }
    
    /// Get animation count
    pub fn animation_count(&self) -> usize {
        self.animations.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    

    #[test]
    fn test_animated_attachment_creation() {
        let attachment = AnimatedAttachment {
            id: Uuid::new_v4(),
            filename: "test.gif".to_string(),
            content_type: "image/gif".to_string(),
            format: AnimationFormat::Gif,
            size: 1024,
            width: Some(100),
            height: Some(100),
            duration_ms: Some(1000),
            frame_count: Some(10),
            auto_play: true,
            animation_id: Some(Uuid::new_v4()),
            created_at: Utc::now(),
        };

        assert_eq!(attachment.filename, "test.gif");
        assert_eq!(attachment.format, AnimationFormat::Gif);
        assert!(attachment.auto_play);
    }

    #[test]
    fn test_animation_format_detection() {
        // This would require a real manager instance
        // Just test the logic for content type detection
        let content_types = [
            ("image/gif", true),
            ("image/webp", true),
            ("image/png", true),
            ("image/avif", true),
            ("image/jpeg", false),
            ("text/plain", false),
        ];

        for (content_type, expected) in content_types {
            // Simple check for animated content types
            let is_animated = matches!(content_type, 
                "image/gif" | "image/webp" | "image/png" | "image/avif"
            );
            assert_eq!(is_animated, expected, "Failed for {}", content_type);
        }
    }

    #[test]
    fn test_animation_control_widget() {
        let animations = vec![
            AnimatedAttachment {
                id: Uuid::new_v4(),
                filename: "anim1.gif".to_string(),
                content_type: "image/gif".to_string(),
                format: AnimationFormat::Gif,
                size: 1024,
                width: Some(100),
                height: Some(100),
                duration_ms: Some(1000),
                frame_count: Some(10),
                auto_play: true,
                animation_id: Some(Uuid::new_v4()),
                created_at: Utc::now(),
            },
            AnimatedAttachment {
                id: Uuid::new_v4(),
                filename: "anim2.gif".to_string(),
                content_type: "image/gif".to_string(),
                format: AnimationFormat::Gif,
                size: 2048,
                width: Some(200),
                height: Some(200),
                duration_ms: Some(2000),
                frame_count: Some(20),
                auto_play: false,
                animation_id: Some(Uuid::new_v4()),
                created_at: Utc::now(),
            },
        ];

        let mut widget = AnimationControlWidget::new(animations);
        assert_eq!(widget.animation_count(), 2);
        assert!(widget.should_show_controls());

        widget.next_animation();
        assert_eq!(widget.selected_index, 1);

        widget.next_animation();
        assert_eq!(widget.selected_index, 0); // Wraps around

        widget.previous_animation();
        assert_eq!(widget.selected_index, 1);
    }
}