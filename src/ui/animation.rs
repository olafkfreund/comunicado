//! Animation support for GIFs and other animated content in terminals
//!
//! This module provides animation playback capabilities for compatible terminals
//! including Kitty, Foot, WezTerm, and others that support graphics protocols.

use crate::ui::graphics::{GraphicsProtocol, ImageRenderer};
use chrono::{DateTime, Utc};
use image::{ImageError, DynamicImage, GenericImageView};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::time::interval;
use uuid::Uuid;

/// Animation-related errors
#[derive(Error, Debug)]
pub enum AnimationError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Image decoding error: {0}")]
    ImageDecoding(#[from] ImageError),
    
    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),
    
    #[error("Animation not found: {0}")]
    NotFound(Uuid),
    
    #[error("Terminal protocol error: {0}")]
    TerminalProtocol(String),
    
    #[error("Frame rate too high: {fps} fps (max: {max})")]
    FrameRateTooHigh { fps: f32, max: f32 },
    
    #[error("Animation too large: {width}x{height} (max: {max_width}x{max_height})")]
    AnimationTooLarge { width: u32, height: u32, max_width: u32, max_height: u32 },
}

pub type AnimationResult<T> = Result<T, AnimationError>;

/// Animation format support
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AnimationFormat {
    Gif,
    WebP,
    Apng,
    Avif,
}

impl AnimationFormat {
    /// Detect format from file extension
    pub fn from_extension(path: &Path) -> Option<Self> {
        match path.extension()?.to_str()?.to_lowercase().as_str() {
            "gif" => Some(Self::Gif),
            "webp" => Some(Self::WebP),
            "png" => Some(Self::Apng), // Could be APNG
            "avif" => Some(Self::Avif),
            _ => None,
        }
    }
    
    /// Check if format supports animation
    pub fn supports_animation(&self) -> bool {
        matches!(self, Self::Gif | Self::WebP | Self::Apng | Self::Avif)
    }
}

/// Individual animation frame
#[derive(Debug, Clone)]
pub struct AnimationFrame {
    pub image: DynamicImage,
    pub delay_ms: u32,
    pub disposal_method: FrameDisposal,
    pub blend_method: FrameBlend,
    pub x_offset: u32,
    pub y_offset: u32,
}

/// Frame disposal methods
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FrameDisposal {
    /// Do not dispose (leave frame)
    None,
    /// Clear to background color
    Background,
    /// Restore to previous frame
    Previous,
}

/// Frame blending methods
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FrameBlend {
    /// Replace pixels
    Source,
    /// Alpha blend with previous
    Over,
}

/// Animation metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimationMetadata {
    pub id: Uuid,
    pub format: AnimationFormat,
    pub width: u32,
    pub height: u32,
    pub frame_count: usize,
    pub duration_ms: u32,
    pub loop_count: Option<u32>, // None = infinite
    pub background_color: Option<[u8; 4]>, // RGBA
    pub created_at: DateTime<Utc>,
    pub file_size: Option<u64>,
    pub source_path: Option<PathBuf>,
}

/// Animation playback state
#[derive(Debug, Clone, PartialEq)]
pub enum AnimationState {
    Stopped,
    Playing,
    Paused,
    Finished,
}

/// Animation playback settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimationSettings {
    pub auto_play: bool,
    pub loop_playback: bool,
    pub respect_frame_timing: bool,
    pub max_fps: f32,
    pub max_size: (u32, u32), // width, height
    pub quality_scaling: f32, // 0.1 to 1.0
    pub enable_smoothing: bool,
    pub background_transparency: bool,
}

impl Default for AnimationSettings {
    fn default() -> Self {
        Self {
            auto_play: false,
            loop_playback: true,
            respect_frame_timing: true,
            max_fps: 30.0,
            max_size: (800, 600),
            quality_scaling: 1.0,
            enable_smoothing: true,
            background_transparency: true,
        }
    }
}

/// Animation decoder for various formats
pub struct AnimationDecoder {
    settings: AnimationSettings,
    #[allow(dead_code)]
    supported_protocols: Vec<GraphicsProtocol>,
}

impl AnimationDecoder {
    /// Create a new animation decoder
    pub fn new(settings: AnimationSettings) -> Self {
        Self {
            settings,
            supported_protocols: GraphicsProtocol::detect_supported(),
        }
    }
    
    /// Check if animation format is supported
    pub fn supports_format(&self, format: &AnimationFormat) -> bool {
        match format {
            AnimationFormat::Gif => true,
            AnimationFormat::WebP => true, // With feature flag
            AnimationFormat::Apng => false, // TODO: Implement
            AnimationFormat::Avif => false, // TODO: Implement
        }
    }
    
    /// Decode animation from file
    pub async fn decode_file(&self, path: &Path) -> AnimationResult<Animation> {
        let format = AnimationFormat::from_extension(path)
            .ok_or_else(|| AnimationError::UnsupportedFormat(
                path.extension().unwrap_or_default().to_string_lossy().to_string()
            ))?;
        
        if !self.supports_format(&format) {
            return Err(AnimationError::UnsupportedFormat(format!("{:?}", format)));
        }
        
        let file_data = tokio::fs::read(path).await?;
        self.decode_bytes(&file_data, format, Some(path.to_path_buf())).await
    }
    
    /// Decode animation from bytes
    pub async fn decode_bytes(
        &self,
        data: &[u8],
        format: AnimationFormat,
        source_path: Option<PathBuf>,
    ) -> AnimationResult<Animation> {
        match format {
            AnimationFormat::Gif => self.decode_gif(data, source_path).await,
            AnimationFormat::WebP => self.decode_webp(data, source_path).await,
            _ => Err(AnimationError::UnsupportedFormat(format!("{:?}", format))),
        }
    }
    
    /// Decode GIF animation
    async fn decode_gif(
        &self,
        data: &[u8],
        source_path: Option<PathBuf>,
    ) -> AnimationResult<Animation> {
        // Simple placeholder implementation for GIF support
        // In a real implementation, this would use a GIF decoding library
        
        // Create a single-frame "animation" from the GIF as a static image
        let image = image::load_from_memory_with_format(data, image::ImageFormat::Gif)?;
        let (width, height) = image.dimensions();
        
        // Check size limits
        if width > self.settings.max_size.0 || height > self.settings.max_size.1 {
            return Err(AnimationError::AnimationTooLarge {
                width,
                height,
                max_width: self.settings.max_size.0,
                max_height: self.settings.max_size.1,
            });
        }
        
        // Create a single frame
        let frame = AnimationFrame {
            image,
            delay_ms: 100, // Default delay
            disposal_method: FrameDisposal::None,
            blend_method: FrameBlend::Source,
            x_offset: 0,
            y_offset: 0,
        };
        
        let metadata = AnimationMetadata {
            id: Uuid::new_v4(),
            format: AnimationFormat::Gif,
            width,
            height,
            frame_count: 1,
            duration_ms: 100,
            loop_count: Some(1),
            background_color: None,
            created_at: Utc::now(),
            file_size: Some(data.len() as u64),
            source_path,
        };
        
        Ok(Animation::new(metadata, vec![frame], self.settings.clone()))
    }
    
    /// Decode WebP animation (placeholder)
    async fn decode_webp(
        &self,
        _data: &[u8],
        _source_path: Option<PathBuf>,
    ) -> AnimationResult<Animation> {
        // TODO: Implement WebP animation decoding
        Err(AnimationError::UnsupportedFormat("WebP animation not yet implemented".to_string()))
    }
}

/// Main animation structure
pub struct Animation {
    pub metadata: AnimationMetadata,
    frames: Vec<AnimationFrame>,
    settings: AnimationSettings,
    
    // Playback state
    current_frame: usize,
    state: AnimationState,
    loop_count: u32,
    last_frame_time: Option<Instant>,
    
    // Rendered frames cache
    rendered_frames: VecDeque<(usize, Vec<u8>)>, // (frame_index, rendered_data)
    max_cache_size: usize,
}

impl Animation {
    /// Create a new animation
    pub fn new(
        metadata: AnimationMetadata,
        frames: Vec<AnimationFrame>,
        settings: AnimationSettings,
    ) -> Self {
        Self {
            metadata,
            frames,
            settings,
            current_frame: 0,
            state: AnimationState::Stopped,
            loop_count: 0,
            last_frame_time: None,
            rendered_frames: VecDeque::new(),
            max_cache_size: 10, // Cache last 10 frames
        }
    }
    
    /// Get animation metadata
    pub fn metadata(&self) -> &AnimationMetadata {
        &self.metadata
    }
    
    /// Get current playback state
    pub fn state(&self) -> AnimationState {
        self.state.clone()
    }
    
    /// Get current frame index
    pub fn current_frame(&self) -> usize {
        self.current_frame
    }
    
    /// Get total frame count
    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }
    
    /// Start playback
    pub fn play(&mut self) {
        if self.state != AnimationState::Playing {
            self.state = AnimationState::Playing;
            self.last_frame_time = Some(Instant::now());
        }
    }
    
    /// Pause playback
    pub fn pause(&mut self) {
        if self.state == AnimationState::Playing {
            self.state = AnimationState::Paused;
        }
    }
    
    /// Stop playback and reset to first frame
    pub fn stop(&mut self) {
        self.state = AnimationState::Stopped;
        self.current_frame = 0;
        self.loop_count = 0;
        self.last_frame_time = None;
    }
    
    /// Seek to specific frame
    pub fn seek_to_frame(&mut self, frame_index: usize) {
        if frame_index < self.frames.len() {
            self.current_frame = frame_index;
            self.last_frame_time = Some(Instant::now());
        }
    }
    
    /// Update animation state and get current frame if changed
    pub fn update(&mut self) -> Option<&AnimationFrame> {
        if self.state != AnimationState::Playing || self.frames.is_empty() {
            return None;
        }
        
        let current_time = Instant::now();
        let should_advance = if let Some(last_time) = self.last_frame_time {
            let elapsed = current_time.duration_since(last_time);
            let current_frame_delay = Duration::from_millis(self.frames[self.current_frame].delay_ms as u64);
            
            elapsed >= current_frame_delay
        } else {
            true // First frame
        };
        
        if should_advance {
            let frame_changed = self.advance_frame();
            self.last_frame_time = Some(current_time);
            
            if frame_changed {
                return Some(&self.frames[self.current_frame]);
            }
        }
        
        None
    }
    
    /// Advance to next frame
    fn advance_frame(&mut self) -> bool {
        let old_frame = self.current_frame;
        
        self.current_frame += 1;
        
        if self.current_frame >= self.frames.len() {
            // End of animation
            if self.settings.loop_playback {
                if let Some(max_loops) = self.metadata.loop_count {
                    self.loop_count += 1;
                    if self.loop_count >= max_loops {
                        self.state = AnimationState::Finished;
                        return false;
                    }
                }
                self.current_frame = 0;
            } else {
                self.state = AnimationState::Finished;
                self.current_frame = self.frames.len() - 1;
                return false;
            }
        }
        
        old_frame != self.current_frame
    }
    
    /// Get current frame
    pub fn current_frame_data(&self) -> Option<&AnimationFrame> {
        self.frames.get(self.current_frame)
    }
    
    /// Render current frame for terminal display
    pub async fn render_current_frame(
        &mut self,
        renderer: &ImageRenderer,
        width: u32,
        height: u32,
    ) -> AnimationResult<Vec<u8>> {
        // Check cache first
        if let Some((_, rendered_data)) = self.rendered_frames
            .iter()
            .find(|(frame_idx, _)| *frame_idx == self.current_frame)
        {
            return Ok(rendered_data.clone());
        }
        
        // Render frame
        if let Some(frame) = self.current_frame_data() {
            let rendered = renderer.render_image(&frame.image, width, height).await
                .map_err(|e| AnimationError::TerminalProtocol(e.to_string()))?;
            
            // Cache rendered frame
            self.rendered_frames.push_back((self.current_frame, rendered.clone()));
            
            // Limit cache size
            while self.rendered_frames.len() > self.max_cache_size {
                self.rendered_frames.pop_front();
            }
            
            Ok(rendered)
        } else {
            Err(AnimationError::NotFound(self.metadata.id))
        }
    }
    
    /// Clear rendered frame cache
    pub fn clear_cache(&mut self) {
        self.rendered_frames.clear();
    }
    
    /// Get playback progress (0.0 to 1.0)
    pub fn progress(&self) -> f32 {
        if self.frames.is_empty() {
            0.0
        } else {
            self.current_frame as f32 / self.frames.len() as f32
        }
    }
    
    /// Get estimated file size reduction with quality scaling
    pub fn estimate_memory_usage(&self) -> u64 {
        let frame_size = (self.metadata.width * self.metadata.height * 4) as u64; // RGBA
        let total_frames = self.frames.len() as u64;
        let quality_factor = self.settings.quality_scaling as u64;
        
        frame_size * total_frames * quality_factor / 100
    }
}

/// Animation manager for handling multiple animations
pub struct AnimationManager {
    animations: Arc<Mutex<Vec<Animation>>>,
    renderer: Arc<ImageRenderer>,
    settings: AnimationSettings,
    decoder: AnimationDecoder,
    update_interval: Duration,
}

impl AnimationManager {
    /// Create a new animation manager
    pub fn new(renderer: Arc<ImageRenderer>, settings: AnimationSettings) -> Self {
        let decoder = AnimationDecoder::new(settings.clone());
        
        Self {
            animations: Arc::new(Mutex::new(Vec::new())),
            renderer,
            settings,
            decoder,
            update_interval: Duration::from_millis(16), // ~60 FPS update rate
        }
    }
    
    /// Load animation from file
    pub async fn load_animation(&self, path: &Path) -> AnimationResult<Uuid> {
        let animation = self.decoder.decode_file(path).await?;
        let id = animation.metadata.id;
        
        {
            let mut animations = self.animations.lock().unwrap();
            animations.push(animation);
        }
        
        Ok(id)
    }
    
    /// Load animation from bytes
    pub async fn load_animation_from_bytes(
        &self,
        data: &[u8],
        format: AnimationFormat,
    ) -> AnimationResult<Uuid> {
        let animation = self.decoder.decode_bytes(data, format, None).await?;
        let id = animation.metadata.id;
        
        {
            let mut animations = self.animations.lock().unwrap();
            animations.push(animation);
        }
        
        Ok(id)
    }
    
    /// Remove animation
    pub fn remove_animation(&self, id: Uuid) -> bool {
        let mut animations = self.animations.lock().unwrap();
        if let Some(pos) = animations.iter().position(|a| a.metadata.id == id) {
            animations.remove(pos);
            true
        } else {
            false
        }
    }
    
    /// Get animation metadata
    pub fn get_metadata(&self, id: Uuid) -> Option<AnimationMetadata> {
        let animations = self.animations.lock().unwrap();
        animations.iter()
            .find(|a| a.metadata.id == id)
            .map(|a| a.metadata.clone())
    }
    
    /// Control animation playback
    pub fn play_animation(&self, id: Uuid) -> bool {
        let mut animations = self.animations.lock().unwrap();
        if let Some(animation) = animations.iter_mut().find(|a| a.metadata.id == id) {
            animation.play();
            true
        } else {
            false
        }
    }
    
    pub fn pause_animation(&self, id: Uuid) -> bool {
        let mut animations = self.animations.lock().unwrap();
        if let Some(animation) = animations.iter_mut().find(|a| a.metadata.id == id) {
            animation.pause();
            true
        } else {
            false
        }
    }
    
    pub fn stop_animation(&self, id: Uuid) -> bool {
        let mut animations = self.animations.lock().unwrap();
        if let Some(animation) = animations.iter_mut().find(|a| a.metadata.id == id) {
            animation.stop();
            true
        } else {
            false
        }
    }
    
    /// Get current frame for rendering
    pub async fn render_animation(
        &self,
        id: Uuid,
        width: u32,
        height: u32,
    ) -> AnimationResult<Option<Vec<u8>>> {
        let mut animations = self.animations.lock().unwrap();
        if let Some(animation) = animations.iter_mut().find(|a| a.metadata.id == id) {
            if animation.state() == AnimationState::Playing {
                animation.update();
            }
            
            if animation.current_frame_data().is_some() {
                let rendered = animation.render_current_frame(&self.renderer, width, height).await?;
                Ok(Some(rendered))
            } else {
                Ok(None)
            }
        } else {
            Err(AnimationError::NotFound(id))
        }
    }
    
    /// Start the animation update loop
    pub async fn start_update_loop(&self) {
        let animations = Arc::clone(&self.animations);
        let mut interval = interval(self.update_interval);
        
        loop {
            interval.tick().await;
            
            let mut animations = animations.lock().unwrap();
            for animation in animations.iter_mut() {
                if animation.state() == AnimationState::Playing {
                    animation.update();
                }
            }
        }
    }
    
    /// Get list of all animations
    pub fn list_animations(&self) -> Vec<AnimationMetadata> {
        let animations = self.animations.lock().unwrap();
        animations.iter().map(|a| a.metadata.clone()).collect()
    }
    
    /// Update global settings
    pub fn update_settings(&mut self, settings: AnimationSettings) {
        self.settings = settings.clone();
        self.decoder = AnimationDecoder::new(settings);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    

    #[test]
    fn test_animation_format_detection() {
        assert_eq!(
            AnimationFormat::from_extension(Path::new("test.gif")),
            Some(AnimationFormat::Gif)
        );
        assert_eq!(
            AnimationFormat::from_extension(Path::new("test.webp")),
            Some(AnimationFormat::WebP)
        );
        assert_eq!(
            AnimationFormat::from_extension(Path::new("test.jpg")),
            None
        );
    }

    #[test]
    fn test_animation_settings_default() {
        let settings = AnimationSettings::default();
        assert!(!settings.auto_play);
        assert!(settings.loop_playback);
        assert_eq!(settings.max_fps, 30.0);
    }

    #[test]
    fn test_animation_metadata_creation() {
        let metadata = AnimationMetadata {
            id: Uuid::new_v4(),
            format: AnimationFormat::Gif,
            width: 100,
            height: 100,
            frame_count: 10,
            duration_ms: 1000,
            loop_count: Some(0),
            background_color: None,
            created_at: Utc::now(),
            file_size: Some(1024),
            source_path: None,
        };

        assert_eq!(metadata.frame_count, 10);
        assert_eq!(metadata.duration_ms, 1000);
    }

    #[tokio::test]
    async fn test_animation_decoder_creation() {
        let settings = AnimationSettings::default();
        let decoder = AnimationDecoder::new(settings);
        
        assert!(decoder.supports_format(&AnimationFormat::Gif));
        assert!(!decoder.supports_format(&AnimationFormat::Apng));
    }
}